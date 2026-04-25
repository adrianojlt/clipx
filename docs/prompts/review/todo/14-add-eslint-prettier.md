# Task 14 - Add ESLint and Prettier

**Severity:** High
**Category:** Tooling
**Depends on:** Nothing - independent, but easier if done before heavy React refactoring

## Why This Is a Problem

The project has no JavaScript linting or formatting tools. This means:
- No enforcement of React best practices (e.g. missing useEffect dependencies)
- Code style will drift as the project grows
- Bugs like unused variables, missing keys in lists, and stale closures go undetected
- Every code editor formats differently

ESLint finds real bugs. Prettier handles formatting. Together they eliminate a whole class of review comments.

## Files to Touch

- `package.json` (add devDependencies and scripts)
- `.eslintrc.json` (create)
- `.prettierrc.json` (create)
- `.prettierignore` (create)

## Step 1 - Install Packages

```bash
pnpm add -D eslint @eslint/js eslint-plugin-react eslint-plugin-react-hooks prettier
```

## Step 2 - Create .eslintrc.json

```json
{
    "env": {
        "browser": true,
        "es2022": true
    },
    "extends": [
        "eslint:recommended",
        "plugin:react/recommended",
        "plugin:react/jsx-runtime",
        "plugin:react-hooks/recommended"
    ],
    "parserOptions": {
        "ecmaVersion": "latest",
        "sourceType": "module",
        "ecmaFeatures": { "jsx": true }
    },
    "plugins": ["react", "react-hooks"],
    "settings": {
        "react": { "version": "detect" }
    },
    "rules": {
        "no-unused-vars": "warn",
        "react-hooks/exhaustive-deps": "warn"
    }
}
```

## Step 3 - Create .prettierrc.json

```json
{
    "semi": true,
    "singleQuote": false,
    "tabWidth": 4,
    "trailingComma": "es5",
    "printWidth": 100
}
```

Match whatever style is already in the codebase (check if the existing code uses single or double quotes, semicolons, etc.).

## Step 4 - Create .prettierignore

```
dist/
node_modules/
src-tauri/
*.json
```

## Step 5 - Add Scripts to package.json

```json
"scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview",
    "tauri": "tauri",
    "lint": "eslint src --ext .jsx,.js",
    "format": "prettier --write src/"
}
```

## Step 6 - Run and Fix Existing Warnings

```bash
pnpm lint
pnpm format
```

ESLint will likely warn about missing `useEffect` dependencies. Fix them one by one. Each warning is worth understanding - some are intentional (the empty dependency array that runs once on mount) and can be suppressed with `// eslint-disable-next-line react-hooks/exhaustive-deps` with a comment explaining why.

## How to Verify

```bash
pnpm lint
```

Should exit with 0 errors. Warnings are acceptable for now. Run again after any code change to confirm no new issues.
