use jumprope::JumpRope;
use rand::prelude::*;
use crate::AgentId;
use crate::list::{Branch, fuzzer_tools, ListCRDT, OpLog};
use crate::list::fuzzer_tools::choose_2;

#[test]
fn random_single_document() {
    let mut rng = SmallRng::seed_from_u64(10);
    let mut doc = ListCRDT::new();

    let agent = doc.get_or_create_agent_id("seph");
    let mut expected_content = JumpRope::new();

    for _i in 0..1000 {
        // eprintln!("i {}", _i);
        // doc.debug_print_stuff();
        fuzzer_tools::make_random_change(&mut doc, Some(&mut expected_content), agent, &mut rng);
        assert_eq!(doc.branch.content, expected_content);
    }

    doc.dbg_check(true);
}

fn merge_fuzz(seed: u64, verbose: bool) {
    let mut rng = SmallRng::seed_from_u64(seed);
    let mut oplog = OpLog::new();
    let mut branches = [Branch::new(), Branch::new(), Branch::new()];

    // Each document will have a different local agent ID. I'm cheating here - just making agent
    // 0 for all of them.
    for i in 0..branches.len() {
        oplog.get_or_create_agent_id(format!("agent {}", i).as_str());
    }

    for _i in 0..300 {
        if verbose { println!("\n\ni {}", _i); }
        // Generate some operations
        for _j in 0..2 {
        // for _j in 0..5 {
            let idx = rng.gen_range(0..branches.len());
            let branch = &mut branches[idx];

            // This should + does also work if we set idx=0 and use the same agent for all changes.
            let v = fuzzer_tools::make_random_change_raw(&mut oplog, branch, None, idx as AgentId, &mut rng);
            // dbg!(opset.iter_range((v..v+1).into()).next().unwrap());

            branch.merge(&oplog, &[v]);
            // make_random_change(doc, None, 0, &mut rng);
            // println!("branch {} content '{}'", idx, &branch.content);
        }

        // Then merge 2 branches at random
        // TODO: Rewrite this to use choose_2.
        let (a_idx, a, b_idx, b) = choose_2(&mut branches, &mut rng);

        if verbose {
            println!("\n\n-----------");
            println!("a content '{}'", a.content);
            println!("b content '{}'", b.content);
            println!("Merging a({}) {:?} and b({}) {:?}", a_idx, &a.version, b_idx, &b.version);
            println!();
        }

        // if _i == 253 {
        //     dbg!(&opset.client_with_localtime);
        // }

        // dbg!(&opset);

        if verbose { println!("Merge b to a: {:?} -> {:?}", &b.version, &a.version); }
        a.merge(&oplog, &b.version);
        if verbose {
            println!("-> a content '{}'\n", a.content);
        }

        if verbose { println!("Merge a to b: {:?} -> {:?}", &a.version, &b.version); }
        b.merge(&oplog, &a.version);
        if verbose {
            println!("-> b content '{}'", b.content);
        }


        // Our frontier should contain everything in the document.

        // a.check(false);
        // b.check(false);

        if a != b {
            println!("Docs {} and {} after {} iterations:", a_idx, b_idx, _i);
            dbg!(&a);
            dbg!(&b);
            panic!("Documents do not match");
        } else {
            if verbose {
                println!("Merge {:?} -> '{}'", &a.version, a.content);
            }
        }

        if _i % 50 == 0 {
            // Every little while, merge everything. This has 2 purposes:
            // 1. It stops the fuzzer being n^2. (Its really unfortunate we need this)
            // And 2. It makes sure n-way merging also works correctly.
            let all_frontier = oplog.version.as_slice();

            for b in branches.iter_mut() {
                b.merge(&oplog, all_frontier);
            }
            for w in branches.windows(2) {
                assert_eq!(w[0].content, w[1].content);
            }
        }

        // for doc in &branches {
        //     doc.check(false);
        // }
    }

    // for doc in &branches {
    //     doc.check(true);
    // }
}

// Included in standard smoke tests.
#[test]
fn fuzz_once_quietly() {
    merge_fuzz(0, false);
}

#[test]
#[ignore]
fn fuzz_once() {
    merge_fuzz(2000 + 32106, true);
}

#[test]
#[ignore]
fn fuzz_merge_forever() {
    for k in 0.. {
        // println!("\n\n*** Iteration {} ***\n", k);
        if k % 100 == 0 {
            println!("Iteration {}", k);
        }
        merge_fuzz(1000000 + k, false);
    }
}