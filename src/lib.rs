//! This is a super fast CRDT implemented in rust. It currently only supports plain text documents
//! but the plan is to support all kinds of data.
//!
//! Diamond types is built on top of two core abstractions:
//!
//! 1. The [Operation Log](list::OpLog)
//! 2. [Branches](list::Branch)
//!
//! A branch is a copy of the document state at some point in time. The most common & useful way to
//! use branches is to make a single branch at the most recent version of the document. When more
//! changes come in, a branch can be moved forward in time by calling [`merge`](list::Branch::merge).
//!
//! Branches in diamond types aren't the same as branches in git. They're a lower level construct.
//! Diamond types doesn't store a list of the active branches in your data set. A branch is much
//! simplier than that - internally its just a temporary in-memory tuple of
//! (version, document state).
//!
//! Branches can change over time by referencing the *Operation Log* (OpLog). The oplog is an
//! append-only log of all the changes which have happened to a document over time. The operation
//! log can be replayed to generate a branch at any point of time within its range.
//!
//! For every operation in the oplog we store a few fields:
//!
//! - What the change actually is (eg *insert 'hi' at position 20*)
//! - Parents (A logical clock of *when* an operation happened)
//! - ID (Agent & Sequence number). The agent can be used to figure out who made the change.
//!
//! ## Example
//!
//! For local edits to an oplog, just use [`oplog.add_insert`](list::OpLog::add_insert) or
//! [`oplog.add_delete_without_content`](list::OpLog::add_delete_without_content):
//!
//! ```
//! use diamond_types::list::*;
//!
//! let mut oplog = OpLog::new();
//! let fred = oplog.get_or_create_agent_id("fred");
//! oplog.add_insert(fred, 0, "abc");
//! oplog.add_delete_without_content(fred, 1..2); // Delete the 'b'
//! ```
//!
//! There are also other methods like [`oplog.add_insert_at`](list::OpLog::add_insert_at) which
//! append a change at some specific point in time. This is useful if you want to append a change to
//! a branch.
//!
//! To create a branch from an oplog, use [`Branch::new` methods](list::Branch::new_at_tip):
//!
//! ```
//! use diamond_types::list::*;
//! let mut oplog = OpLog::new();
//! // ...
//! let mut branch = Branch::new_at_tip(&oplog);
//! // Equivalent to let mut branch = Branch::new_at_local_version(&oplog, oplog.get_local_version());
//! println!("branch content {}", branch.content().to_string());
//! ```
//!
//! Once a branch has been created, you can merge new changes using [`branch.merge`](list::Branch::merge):
//!
//! ```
//! use diamond_types::list::*;
//! let mut oplog = OpLog::new();
//! // ...
//! let mut branch = Branch::new_at_tip(&oplog);
//! let george = oplog.get_or_create_agent_id("george");
//! oplog.add_insert(george, 0, "asdf");
//! branch.merge(&oplog, oplog.local_version_ref());
//! ```
//!
//! If you aren't using branches, you can use the simplified [`ListCRDT` API](list::ListCRDT). The
//! ListCRDT struct simply wraps an oplog and a branch together so you don't need to muck about
//! with manual merging. This API is also slightly faster.
//!
//! I'm holding off on adding examples using this API for now because the API is in flux. TODO: Fix!
//!
//!
//! ## Consuming IDs
//!
//! The ID of a change is made up of an agent ID (usually an opaque string) and a sequence number.
//! Each successive change from the same agent will use the next sequence number - eg: (*fred*, 0),
//! (*fred*, 1), (*fred*, 2), etc.
//!
//! But its important to note what constitutes a change! In diamond types, every inserted character
//! or deleted character increments (consumes) a sequence number. Typing a run of characters one at
//! a time is indistinguishable from pasting the same run of characters all at once.
//!
//! Note that this is a departure from other CRDTs. Automerge does not work this way.
//!
//! For example,
//!
//! ```
//! use diamond_types::list::*;
//! let mut oplog = OpLog::new();
//! let fred = oplog.get_or_create_agent_id("fred");
//! oplog.add_insert(fred, 0, "a");
//! oplog.add_insert(fred, 1, "b");
//! oplog.add_insert(fred, 2, "c");
//! ```
//!
//! Produces an identical oplog to this:
//!
//! ```
//! use diamond_types::list::*;
//! let mut oplog = OpLog::new();
//! let fred = oplog.get_or_create_agent_id("fred");
//! oplog.add_insert(fred, 0, "abc");
//! ```
//!
//! Diamond types does this by very aggressively run-length encoding everything it can whenever
//! possible.
//!
//! ### Warning: Do not reuse IDs 💣!
//!
//! Every ID in diamond types *must be unique*. If two operations are created with the same ID,
//! peers will only merge one of them - and the document state will diverge. This is really bad!
//!
//! Its tempting to reuse agent IDs because they waste disk space. But there's lots of ways to
//! introduce subtle bugs if you try. Disk space is cheap. Bugs are expensive.
//!
//! I recommend instead just generating a new agent ID in every editing session. So, in a text
//! editor, generate an ID in memory when the user opens the document. Don't save the ID to disk.
//! Just discard it when the user's editing session ends.
//!
//!
//! ### Aside on atomic transactions
//!
//! Grouping changes in atomic blocks is out of the scope of diamond types. But you can implement it
//! in the code you write which consumes diamond types. Briefly, either:
//!
//! 1. Make all the changes you want to make atomically in diamond types, but delay sending those
//! changes over the network until you're ready, or
//! 2. Add a special commit message to your network protocol which "commits" marks when a set of
//! operations in the oplog is safe to merge.
//!
//! Diamond types does not (yet) support deleting operations from the oplog. If this matters to you,
//! please start open an issue about it.
//!
//!
//! ## Parents
//!
//! The parents list names the version of the document right before it was changed. An new,
//! empty document always has the version of *ROOT*. After an operation has happened, the version of
//! the document is the same as that operation's ID.
//!
//! Sometimes changes are concurrent. This can happen in realtime - for example, two users type in a
//! collaborative document at the same time. Or it can happen asyncronously - for example, two users
//! edit two different branches, and later merge their results. We can describe what happened with
//! a *time DAG*, where each change is represented by a node in a DAG (Directed Acyclic Graph).
//! Edges represent the *directly after* relationship. See [INTERNALS.md](INTERNALS.md) in this
//! repository for more theoretical information.
//!
//! For example, in this time DAG operations `a` and `b` are concurrent:
//!
//! ```text
//!   ROOT
//!   / \
//!  a   b
//!   \ /
//!    c
//! ```
//!
//! Concurrent changes have some repercussions for the oplog:
//!
//! - The order of changes in the oplog isn't canonical. Other peers may have oplogs with a
//! different order. This is fine. DT uses "local time" numbers heavily internally - which refer to
//! the local index of a change, as if it were stored in an array. But these indexes cannot be
//! shared with other peers. However, the order of changes must always obey the partial order of
//! chronology. If operation A happened before operation B, they must maintain that relative order
//! in the oplog. In the diagram above, the operations could be stored in the order `[a, b, c]` or
//! `[b, a, c]` but not `[a, c, b]` because `c` comes after both `a` and `b`.
//! - We represent a point in time in the oplog using a *list* of (agent, seq) pairs. This list
//! usually only contains one entry - which is the ID of the preceding operation. But sometimes
//! we need to merge two threads of history together. In this case, the parents list names all
//! immediate predecessors. In the diagram above, operation `c` has a parents list of both `a` and
//! `b`.
//!
//! Unlike git (and some other CRDTs), diamond types represents merges *implicitly*. We don't create
//! a special node in the time DAG for merges. Merges simply happen whenever an operation has
//! multiple parents.

#![allow(clippy::module_inception)]

extern crate core;

use smallvec::SmallVec;
use smartstring::alias::String as SmartString;
use crate::dtrange::DTRange;
use crate::rle::{KVPair, RleVec};

pub mod list;
mod rle;
mod dtrange;
mod unicount;
mod remotespan;
mod rev_range;
mod history;
mod frontier;
mod history_tools;

pub type AgentId = u32;
const ROOT_AGENT: AgentId = AgentId::MAX;
const ROOT_TIME: usize = usize::MAX;

// TODO: Consider changing this to u64 to add support for very long lived documents even on 32 bit
// systems.
pub type Time = usize;

/// A LocalVersion is a set of local Time values which point at the set of changes with no children
/// at this point in time. When there's a single writer this will
/// always just be the last order we've seen.
///
/// This is never empty.
///
/// At the start of time (when there are no changes), LocalVersion is usize::max (which is the root
/// order).
pub type LocalVersion = SmallVec<[Time; 2]>;

#[derive(Clone, Debug)]
struct ClientData {
    /// Used to map from client's name / hash to its numerical ID.
    name: SmartString,

    /// This is a packed RLE in-order list of all operations from this client.
    ///
    /// Each entry in this list is grounded at the client's sequence number and maps to the span of
    /// local time entries.
    ///
    /// A single agent ID might be used to modify multiple concurrent branches. Because of this, and
    /// the propensity of diamond types to reorder operations for performance, the
    /// time spans here will *almost* always (but not always) be monotonically increasing. Eg, they
    /// might be ordered as (0, 2, 1). This will only happen when changes are concurrent. The order
    /// of time spans must always obey the partial order of changes. But it will not necessarily
    /// agree with the order amongst time spans.
    item_times: RleVec<KVPair<DTRange>>,
}
