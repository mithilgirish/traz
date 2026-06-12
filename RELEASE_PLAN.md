# traz v0.1.0 Release Plan

**Target Release Date:** Next Tuesday
**Current Version:** v0.1.0

This document outlines the exact step-by-step process we will follow to set up automated, cross-platform distribution (NPM, Homebrew, and standalone installers) and successfully launch `traz` to the public.

---

## Phase 1: Prerequisites (To do before Tuesday)

These are the one-time manual steps required to connect the automated GitHub Actions to the external package managers.

### 1. NPM Setup
- [ ] Go to [npmjs.com](https://www.npmjs.com/) and create a free account (if you don't have one).
- [ ] Navigate to your NPM Account Settings -> Access Tokens.
- [ ] Click **Generate New Token** (select "Automation" type).
- [ ] Copy the token.
- [ ] Go to your `traz` GitHub Repository -> Settings -> Secrets and variables -> Actions.
- [ ] Click **New repository secret**, name it `NPM_TOKEN`, and paste the token.

### 2. Homebrew Setup
- [ ] Go to your GitHub profile and create a new repository.
- [ ] Name it **exactly**: `homebrew-traz`.
- [ ] Ensure the repository is **Public**.
- [ ] Do not initialize it with a README or `.gitignore` (leave it completely empty).
- [ ] Go to your main `traz` GitHub Repository -> Settings -> Developer settings -> Personal access tokens (Classic).
- [ ] Generate a token with the `repo` scope.
- [ ] Add it as a repository secret named `HOMEBREW_GITHUB_TOKEN` in your main `traz` repo.

---

## Phase 2: Configuration Generation (We can do this today or Monday)

Once the prerequisites are ready, we will generate the `cargo-dist` configuration files.

- [ ] Install `cargo-dist` locally on this machine:
  ```bash
  cargo install cargo-dist
  ```
- [ ] Initialize the distribution configuration:
  ```bash
  cargo dist init
  ```
  *During this interactive prompt, we will enable:*
  * NPM package generation
  * Homebrew tap generation (pointing to `mithilgirish/homebrew-traz`)
  * Standalone Shell & PowerShell installer generation
- [ ] Commit the newly generated `.github/workflows/release.yml` and `Cargo.toml` changes:
  ```bash
  git add .
  git commit -m "chore: setup automated distribution pipelines via cargo-dist"
  git push
  ```

---

## Phase 3: Documentation Website Setup (traz.mithilgirish.dev)

A dedicated, highly polished documentation website will build massive credibility for the project. We are moving forward with **VitePress** using a "Monorepo" strategy (keeping the site inside the same repository).

### 1. Build the Site with VitePress (Monday)
- [ ] Scaffold VitePress directly inside the existing `docs/` directory (`npm add -D vitepress` and add `.vitepress/config.js`).
- [ ] Connect the existing Markdown files (`QUICKSTART.md`, `AGENT_INTEGRATION.md`, etc.) to the site's sidebar navigation.
- [ ] Configure the site metadata with the official logo and title.
- [ ] Test the site locally (`npx vitepress dev docs`).

### 2. Deploy to Vercel (Monday)
- [ ] Connect your GitHub repository to Vercel (or GitHub Pages/Netlify).
- [ ] Set the framework preset to "VitePress" and the root directory as `docs/`.
- [ ] Deploy the site.
- [ ] In your Vercel project settings, add the custom domain: `traz.mithilgirish.dev`.

### 3. DNS Configuration (Monday)
- [ ] Log in to your domain registrar (where you manage `mithilgirish.dev`).
- [ ] Add a `CNAME` record for `traz` pointing to `cname.vercel-dns.com` (or the target provided by your host).

---

## Phase 4: Launch Day (Next Tuesday)

With the pipeline configured and the docs live, releasing the software takes less than a minute.

### 1. Final Polish
- [ ] Ensure the `README.md` is fully updated with the new installation commands (`npm install -g traz`, `brew install mithilgirish/traz/traz`).
- [ ] Ensure `CHANGELOG.md` has the final notes for `v0.1.0`.
- [ ] Ensure the `README.md` links prominently to `https://traz.mithilgirish.dev`.

### 2. The Release Trigger
- [ ] Create an annotated git tag for the release version:
  ```bash
  git tag -a v0.1.0 -m "Release v0.1.0"
  ```
- [ ] Push the tag to GitHub:
  ```bash
  git push origin v0.1.0
  ```

### 3. Monitoring
- [ ] Go to the "Actions" tab on your GitHub repository.
- [ ] Watch the `Release` workflow. It will:
  1. Compile Mac, Linux, and Windows binaries in the cloud.
  2. Upload them to the GitHub Releases page.
  3. Publish the NPM wrapper package using your `NPM_TOKEN`.
  4. Push the Homebrew formula to your `homebrew-traz` repository.

### 4. Post-Launch Verification
- [ ] Test the `npm install -g traz` command locally to verify it works.
- [ ] Post your launch announcement!
