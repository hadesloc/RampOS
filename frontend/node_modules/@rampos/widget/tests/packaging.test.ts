import { describe, it, expect } from 'vitest';
import * as fs from 'fs';
import * as path from 'path';

const ROOT = path.resolve(__dirname, '..');

function stripJsonComments(text: string): string {
  return text.replace(/\/\*[\s\S]*?\*\/|\/\/.*/g, '');
}

function readJSON(filePath: string): Record<string, unknown> {
  const raw = fs.readFileSync(filePath, 'utf-8');
  return JSON.parse(stripJsonComments(raw));
}

describe('F12 Widget SDK - Packaging & Distribution', () => {
  const pkg = readJSON(path.join(ROOT, 'package.json')) as Record<string, unknown>;

  // ---- package.json required fields ----
  describe('package.json required fields', () => {
    it('has name field set to @rampos/widget', () => {
      expect(pkg.name).toBe('@rampos/widget');
    });

    it('has version following semver format', () => {
      expect(pkg.version).toBeDefined();
      expect(typeof pkg.version).toBe('string');
      expect(pkg.version).toMatch(/^\d+\.\d+\.\d+(-[\w.]+)?$/);
    });

    it('has main entry point for CJS consumers', () => {
      expect(pkg.main).toBeDefined();
      expect(typeof pkg.main).toBe('string');
      expect(pkg.main as string).toMatch(/\.js$/);
    });

    it('has module entry point for ESM consumers', () => {
      expect(pkg.module).toBeDefined();
      expect(typeof pkg.module).toBe('string');
      expect(pkg.module as string).toMatch(/\.js$/);
    });

    it('has types entry point for TypeScript declaration files', () => {
      expect(pkg.types).toBeDefined();
      expect(typeof pkg.types).toBe('string');
      expect(pkg.types as string).toMatch(/\.d\.ts$/);
    });

    it('has license field set', () => {
      expect(pkg.license).toBeDefined();
      expect(typeof pkg.license).toBe('string');
      expect((pkg.license as string).length).toBeGreaterThan(0);
    });
  });

  // ---- files & distribution ----
  describe('distribution configuration', () => {
    it('files array includes dist directory', () => {
      expect(pkg.files).toBeDefined();
      expect(Array.isArray(pkg.files)).toBe(true);
      expect(pkg.files as string[]).toContain('dist');
    });

    it('publishConfig is set to public access', () => {
      expect(pkg.publishConfig).toBeDefined();
      expect((pkg.publishConfig as Record<string, unknown>).access).toBe('public');
    });

    it('has keywords for discoverability', () => {
      expect(pkg.keywords).toBeDefined();
      expect(Array.isArray(pkg.keywords)).toBe(true);
      expect((pkg.keywords as string[]).length).toBeGreaterThan(0);
      expect(pkg.keywords as string[]).toContain('widget');
    });
  });

  // ---- scripts ----
  describe('package.json scripts', () => {
    const scripts = pkg.scripts as Record<string, string>;

    it('has build script using vite', () => {
      expect(scripts.build).toBeDefined();
      expect(scripts.build).toContain('vite build');
    });

    it('has test script using vitest', () => {
      expect(scripts.test).toBeDefined();
      expect(scripts.test).toContain('vitest');
    });

    it('has type-check script', () => {
      expect(scripts['type-check']).toBeDefined();
      expect(scripts['type-check']).toContain('tsc');
    });
  });

  // ---- peer dependencies vs dev dependencies ----
  describe('dependency configuration', () => {
    it('react is a peerDependency, not a direct dependency', () => {
      const peerDeps = pkg.peerDependencies as Record<string, string> | undefined;
      const deps = pkg.dependencies as Record<string, string> | undefined;

      expect(peerDeps).toBeDefined();
      expect(peerDeps!.react).toBeDefined();
      // react should NOT be in direct dependencies for a library
      expect(deps?.react).toBeUndefined();
    });

    it('react-dom is a peerDependency, not a direct dependency', () => {
      const peerDeps = pkg.peerDependencies as Record<string, string> | undefined;
      const deps = pkg.dependencies as Record<string, string> | undefined;

      expect(peerDeps!['react-dom']).toBeDefined();
      expect(deps?.['react-dom']).toBeUndefined();
    });

    it('devDependencies does not leak into dependencies', () => {
      const deps = pkg.dependencies as Record<string, string> | undefined;
      const devDeps = pkg.devDependencies as Record<string, string> | undefined;

      // vitest, eslint, jsdom should be devDeps only
      expect(deps?.vitest).toBeUndefined();
      expect(deps?.eslint).toBeUndefined();
      expect(deps?.jsdom).toBeUndefined();
      // Confirm they exist in devDeps
      expect(devDeps?.vitest).toBeDefined();
    });
  });

  // ---- entry point files exist ----
  describe('entry point source files exist', () => {
    it('main library entry exists (src/index.ts)', () => {
      expect(fs.existsSync(path.join(ROOT, 'src/index.ts'))).toBe(true);
    });

    it('embed entry exists (src/embed.ts)', () => {
      expect(fs.existsSync(path.join(ROOT, 'src/embed.ts'))).toBe(true);
    });

    it('CDN entry exists (src/cdn.ts)', () => {
      expect(fs.existsSync(path.join(ROOT, 'src/cdn.ts'))).toBe(true);
    });

    it('types barrel exists (src/types/index.ts)', () => {
      expect(fs.existsSync(path.join(ROOT, 'src/types/index.ts'))).toBe(true);
    });
  });

  // ---- Vite build config ----
  describe('Vite build configuration', () => {
    it('main vite.config.ts exists', () => {
      expect(fs.existsSync(path.join(ROOT, 'vite.config.ts'))).toBe(true);
    });

    it('embed vite config exists (vite.embed.config.ts)', () => {
      expect(fs.existsSync(path.join(ROOT, 'vite.embed.config.ts'))).toBe(true);
    });

    it('embed config uses IIFE format with RampOSWidget library name', () => {
      const configContent = fs.readFileSync(
        path.join(ROOT, 'vite.embed.config.ts'),
        'utf-8'
      );
      expect(configContent).toContain("name: 'RampOSWidget'");
      expect(configContent).toContain("'iife'");
    });

    it('main config externalizes react for library consumers', () => {
      const configContent = fs.readFileSync(
        path.join(ROOT, 'vite.config.ts'),
        'utf-8'
      );
      expect(configContent).toContain("'react'");
      expect(configContent).toContain("'react-dom'");
      expect(configContent).toContain('external');
    });

    it('embed config has no external dependencies (self-contained)', () => {
      const configContent = fs.readFileSync(
        path.join(ROOT, 'vite.embed.config.ts'),
        'utf-8'
      );
      expect(configContent).toContain('external: []');
    });
  });

  // ---- TypeScript configuration ----
  describe('TypeScript configuration', () => {
    it('tsconfig.json exists', () => {
      expect(fs.existsSync(path.join(ROOT, 'tsconfig.json'))).toBe(true);
    });

    it('tsconfig targets ES2020 or higher', () => {
      const tsconfig = readJSON(path.join(ROOT, 'tsconfig.json'));
      const compilerOptions = tsconfig.compilerOptions as Record<string, unknown>;
      expect(compilerOptions.target).toBeDefined();
      const target = (compilerOptions.target as string).toUpperCase();
      // ES2020, ES2021, ES2022, ESNext are all valid modern targets
      expect(target).toMatch(/^ES20(2[0-9]|[3-9]\d)|ESNEXT$/);
    });

    it('strict mode is enabled', () => {
      const tsconfig = readJSON(path.join(ROOT, 'tsconfig.json'));
      const compilerOptions = tsconfig.compilerOptions as Record<string, unknown>;
      expect(compilerOptions.strict).toBe(true);
    });

    it('JSX is configured for react-jsx', () => {
      const tsconfig = readJSON(path.join(ROOT, 'tsconfig.json'));
      const compilerOptions = tsconfig.compilerOptions as Record<string, unknown>;
      expect(compilerOptions.jsx).toBe('react-jsx');
    });
  });

  // ---- Public API stability ----
  describe('public API exports stability', () => {
    it('index.ts exports React components', () => {
      const indexContent = fs.readFileSync(
        path.join(ROOT, 'src/index.ts'),
        'utf-8'
      );
      expect(indexContent).toContain('RampOSCheckout');
      expect(indexContent).toContain('RampOSKYC');
      expect(indexContent).toContain('RampOSWallet');
    });

    it('index.ts exports utility classes', () => {
      const indexContent = fs.readFileSync(
        path.join(ROOT, 'src/index.ts'),
        'utf-8'
      );
      expect(indexContent).toContain('RampOSEventEmitter');
      expect(indexContent).toContain('RampOSApiClient');
    });

    it('index.ts exports type definitions', () => {
      const indexContent = fs.readFileSync(
        path.join(ROOT, 'src/index.ts'),
        'utf-8'
      );
      expect(indexContent).toContain('CheckoutConfig');
      expect(indexContent).toContain('KYCConfig');
      expect(indexContent).toContain('WalletConfig');
      expect(indexContent).toContain('WidgetTheme');
      expect(indexContent).toContain('WidgetEventType');
    });

    it('index.ts exports DEFAULT_THEME constant', () => {
      const indexContent = fs.readFileSync(
        path.join(ROOT, 'src/index.ts'),
        'utf-8'
      );
      expect(indexContent).toContain('DEFAULT_THEME');
    });

    it('embed.ts exports RampOSWidget with required static API', () => {
      const embedContent = fs.readFileSync(
        path.join(ROOT, 'src/embed.ts'),
        'utf-8'
      );
      expect(embedContent).toContain('init');
      expect(embedContent).toContain('destroy');
      expect(embedContent).toContain('destroyAll');
      expect(embedContent).toContain('getInstances');
      expect(embedContent).toContain('version');
    });
  });

  // ---- CSP compatibility ----
  describe('CSP compatibility', () => {
    it('embed.ts does not use eval()', () => {
      const content = fs.readFileSync(
        path.join(ROOT, 'src/embed.ts'),
        'utf-8'
      );
      // Check for direct eval calls (not mentions in strings/comments)
      const lines = content.split('\n').filter(
        (l) => !l.trim().startsWith('//')
      );
      const codeOnly = lines.join('\n');
      expect(codeOnly).not.toMatch(/\beval\s*\(/);
    });

    it('embed.ts does not use new Function()', () => {
      const content = fs.readFileSync(
        path.join(ROOT, 'src/embed.ts'),
        'utf-8'
      );
      const lines = content.split('\n').filter(
        (l) => !l.trim().startsWith('//')
      );
      const codeOnly = lines.join('\n');
      expect(codeOnly).not.toMatch(/new\s+Function\s*\(/);
    });

    it('index.ts does not use eval()', () => {
      const content = fs.readFileSync(
        path.join(ROOT, 'src/index.ts'),
        'utf-8'
      );
      const lines = content.split('\n').filter(
        (l) => !l.trim().startsWith('//')
      );
      const codeOnly = lines.join('\n');
      expect(codeOnly).not.toMatch(/\beval\s*\(/);
    });
  });

  // ---- No circular dependency in core exports ----
  describe('no circular dependencies in main modules', () => {
    it('embed.ts does not import from index.ts (avoiding circular)', () => {
      const content = fs.readFileSync(
        path.join(ROOT, 'src/embed.ts'),
        'utf-8'
      );
      // embed should import from specific sub-modules, not the barrel index
      expect(content).not.toMatch(/from\s+['"]\.\/(index|\.)['"]/);
    });

    it('cdn.ts does not import from index.ts (avoiding circular)', () => {
      const content = fs.readFileSync(
        path.join(ROOT, 'src/cdn.ts'),
        'utf-8'
      );
      expect(content).not.toMatch(/from\s+['"]\.\/(index|\.)['"]/);
    });

    it('types/index.ts does not import from parent src/index.ts', () => {
      const content = fs.readFileSync(
        path.join(ROOT, 'src/types/index.ts'),
        'utf-8'
      );
      expect(content).not.toMatch(/from\s+['"]\.\.\/(index|\.)['"]/);
    });
  });
});
