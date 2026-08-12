// ESLint 9 flat config for the BenchMesh frontend (TypeScript + React).
//
// Pragmatic ruleset for an existing ~12.5k LOC codebase: the goal is a passing
// gate that catches genuine bugs (unused imports, broken hook deps), not a
// purity campaign. Rules that fire hundreds of times on established idioms
// are downgraded or disabled below with a short rationale.
import tseslint from 'typescript-eslint';
import reactHooks from 'eslint-plugin-react-hooks';

export default tseslint.config(
  {
    ignores: ['dist', 'node_modules', 'coverage', 'playwright-report', 'test-results'],
  },
  // Non-type-checked recommended config (type-aware linting is too slow/noisy
  // for a gate on this codebase right now).
  ...tseslint.configs.recommended,
  reactHooks.configs['recommended-latest'],
  {
    rules: {
      // Existing codebase uses `any` liberally at API/WebSocket boundaries.
      '@typescript-eslint/no-explicit-any': 'off',
      // Allow intentionally-unused args/vars when prefixed with `_`.
      '@typescript-eslint/no-unused-vars': [
        'error',
        {
          argsIgnorePattern: '^_',
          varsIgnorePattern: '^_',
          caughtErrorsIgnorePattern: '^_',
        },
      ],
    },
  }
);
