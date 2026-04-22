import typescript from '@rollup/plugin-typescript';
import nodeResolve from '@rollup/plugin-node-resolve';
import terser from '@rollup/plugin-terser';

const isProd = process.env.NODE_ENV !== 'development';
const banner = `/*! @js-monitor/sdk v1.0.0 | MIT License */`;

const plugins = [
  nodeResolve({ browser: true }),
  typescript({
    tsconfig: './tsconfig.json',
    sourceMap: true,
    inlineSources: !isProd,
  }),
];

if (isProd) {
  plugins.push(
    terser({
      format: { comments: /^!/ },
      compress: { drop_console: false, drop_debugger: true },
    })
  );
}

export default {
  input: 'src/index.ts',
  output: [
    {
      file: 'build/sdk.umd.js',
      format: 'umd',
      name: 'Monitor',
      sourcemap: true,
      banner,
      exports: 'named',
    },
    {
      file: 'build/sdk.esm.js',
      format: 'es',
      sourcemap: true,
      banner,
    },
    {
      file: 'build/sdk.iife.js',
      format: 'iife',
      name: 'Monitor',
      sourcemap: true,
      banner,
      exports: 'named',
    },
  ],
  plugins,
};
