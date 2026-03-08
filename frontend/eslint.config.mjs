import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { FlatCompat } from '@eslint/eslintrc';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const compat = new FlatCompat({
  baseDirectory: __dirname,
});

const config = [
  {
    ignores: ['**/.next/**', '**/node_modules/**'],
  },
  ...compat.extends('next/core-web-vitals'),
  {
    files: ['**/*.test.ts', '**/*.test.tsx', '**/__tests__/**/*.{ts,tsx}'],
    rules: {
      'react/display-name': 'off',
      '@next/next/no-html-link-for-pages': 'off',
      'jsx-a11y/role-has-required-aria-props': 'off',
    },
  },
];

export default config;
