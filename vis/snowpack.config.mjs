/** @type {import("snowpack").SnowpackUserConfig } */
export default {
  mount: {
    public: {url: '/', static: true},
    src: {url: '/dist'},
  },
  plugins: [
    '@snowpack/plugin-svelte',
    '@snowpack/plugin-dotenv',
    [
      '@snowpack/plugin-typescript',
      {
        /* Yarn PnP workaround: see https://www.npmjs.com/package/@snowpack/plugin-typescript */
        ...(process.versions.pnp ? {tsc: 'yarn pnpify tsc'} : {}),
      },
    ],
    [
      '@emily-curry/snowpack-plugin-wasm-pack',
      {
        projectPath: '../crates/diamond-wasm'
      }
    ]
  ],
  routes: [
    /* Enable an SPA Fallback in development: */
    // {"match": "routes", "src": ".*", "dest": "/index.html"},
  ],
  optimize: {
    /* Example: Bundle your final build: */
    "bundle": true,
  },
  packageOptions: {

  },
  devOptions: {

  },
  buildOptions: {
    baseUrl: '.'
  },
};
