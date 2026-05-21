```markdown
# ruvector Development Patterns

> Auto-generated skill from repository analysis

## Overview

This skill teaches the core development conventions and workflows for the `ruvector` Rust codebase. It covers file organization, code style, commit patterns, and the main automation workflows for package release, synchronization, and CI hardening. Whether contributing code or maintaining the repository, following these patterns ensures consistency, reliability, and security across the project.

## Coding Conventions

- **File Naming:**  
  Use camelCase for file and module names.  
  _Example:_  
  ```
  src/vectorMath.rs
  src/fastTransform.rs
  ```

- **Import Style:**  
  Use relative imports for referencing modules within the project.  
  _Example:_  
  ```rust
  mod utils;
  use crate::vectorMath::Vector;
  ```

- **Export Style:**  
  Use named exports for modules and functions.  
  _Example:_  
  ```rust
  pub struct Vector { /* ... */ }
  pub fn dot(a: &Vector, b: &Vector) -> f64 { /* ... */ }
  ```

- **Commit Patterns:**  
  - Use [Conventional Commits](https://www.conventionalcommits.org/).
  - Prefixes: `chore`, `fix`, `style`, `ci`
  - Commit messages average ~59 characters.
  - _Example:_  
    ```
    fix: correct vector normalization edge case
    chore: update dependencies for security patch
    ```

## Workflows

### npm-package-version-bump-and-release
**Trigger:** When a new feature or fix needs to be released to npm consumers.  
**Command:** `/release-npm`

1. Update version numbers in all relevant `package.json` files:
    - `npm/packages/router/package.json`
    - `npm/packages/router-darwin-arm64/package.json`
    - `npm/packages/router-darwin-x64/package.json`
    - `npm/packages/router-linux-arm64-gnu/package.json`
    - `npm/packages/router-linux-x64-gnu/package.json`
    - `npm/packages/router-win32-x64-msvc/package.json`
2. Commit the version bump with a message such as:
    ```
    chore: bump npm package versions for release
    ```
3. Push changes. CI will publish to npm after the tag is cut.

---

### sync-readme-and-package-json-to-published-npm
**Trigger:** When a package is published to npm but the updated files were not committed to git.  
**Command:** `/sync-npm-package`

1. Verify the contents of the published npm package.
2. Update `README.md` and `package.json` in the corresponding `npm/packages/*` directory to match the npm registry.
3. Commit the changes with a message like:
    ```
    chore: sync README and package.json with published npm package
    ```
4. Push to update the repository state.

---

### ci-guard-and-supply-chain-hardening
**Trigger:** When new supply chain security layers or regression guards are added or existing ones are updated.  
**Command:** `/add-ci-guard`

1. Add or update GitHub Actions workflow files for CI checks in `.github/workflows/*.yml`.
2. Update dependency policy files (e.g., `deny.toml`).
3. Update lockfiles and package manifests as needed:
    - `npm/package-lock.json`
    - `npm/package.json`
4. Commit all CI and policy changes with a message such as:
    ```
    ci: update CI workflows and dependency policies
    ```
5. Push to enforce new CI and security policies.

---

## Testing Patterns

- **Framework:** Not explicitly detected; likely uses Rust's built-in test framework.
- **File Pattern:** Test files use the `*.test.*` naming convention.
  - _Example:_ `vectorMath.test.rs`
- **Writing Tests:**  
  Use Rust's `#[cfg(test)]` and `#[test]` attributes.  
  _Example:_  
  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;

      #[test]
      fn test_vector_addition() {
          let a = Vector::new(1.0, 2.0);
          let b = Vector::new(3.0, 4.0);
          assert_eq!(a + b, Vector::new(4.0, 6.0));
      }
  }
  ```

## Commands

| Command           | Purpose                                                        |
|-------------------|----------------------------------------------------------------|
| /release-npm      | Bump npm package versions and trigger a release                |
| /sync-npm-package | Sync README and package.json with the published npm package    |
| /add-ci-guard     | Add or update CI workflows and supply chain security policies  |
```