# ADDON_SPEC — LazyDesktop Addon System Design Specification

**Status:** Design specification (Phase 1 deliverable)
**Author:** Principal Systems Architect / Senior Production Engineer
**Scope:** Complete design of the LazyDesktop addon subsystem: Rust security
core, C ABI FFI, C++/Qt Lua host, package formats, security model, threading,
error model, and testing strategy.

**Priority order (governs every design decision):**
1. Memory safety
2. Security
3. Correctness
4. Maintainability
5. Testability
6. Extensibility
7. Performance
8. Convenience

---

## Table of Contents

1. [Overview and Goals](#1-overview-and-goals)
2. [Layered Architecture](#2-layered-architecture)
3. [Separation of Concerns](#3-separation-of-concerns)
4. [Repository Layout](#4-repository-layout)
5. [Addon Package Format](#5-addon-package-format)
6. [Configuration Schema (config.toml)](#6-configuration-schema-configtoml)
7. [Ignore System (.lzdignore)](#7-ignore-system-lzdignore)
8. [Security Model](#8-security-model)
9. [Rust Core Design](#9-rust-core-design)
10. [Provider Architecture](#10-provider-architecture)
11. [FFI ABI Design](#11-ffi-abi-design)
12. [Error Model](#12-error-model)
13. [Lua Execution Model](#13-lua-execution-model)
14. [C++/Qt Host Design](#14-cqt-host-design)
15. [Threading Model](#15-threading-model)
16. [Lifecycle Models](#16-lifecycle-models)
17. [Asset Handling Pipeline](#17-asset-handling-pipeline)
18. [Configuration and Settings Integration](#18-configuration-and-settings-integration)
19. [Build Integration](#19-build-integration)
20. [Testing Strategy](#20-testing-strategy)
21. [Extensibility Roadmap](#21-extensibility-roadmap)
22. [Non-Goals (v1)](#22-non-goals-v1)
23. [Phase 2 Implementation Checklist](#23-phase-2-implementation-checklist)

---

## 1. Overview and Goals

LazyDesktop gains a **scriptable addon system**. Addons are small, packaged
bundles (`.zip`, `.lzd`, or a raw directory) that declare metadata in TOML and
execute Lua scripts inside a sandboxed runtime. Addons can contribute menu
commands, react to application events, show notifications, read validated
package assets, and use a restricted host API — they cannot touch the
filesystem, network, or host internals directly.

### 1.1 Goals

- **Memory safe by construction**: the security boundary is Rust; all package
  parsing, validation, and resource access happens in Rust with `unsafe`
  confined to the FFI export module.
- **Untrusted input is never trusted**: every archive, manifest, path, and
  script is treated as hostile until validated.
- **Deterministic**: fixed ordering, stable error taxonomy, no ambient state
  leakage between addons.
- **Swappable providers**: a trait-based provider seam (`LocalProvider` now,
  `RemoteProvider` later) that requires zero UI changes.
- **Testable**: hermetic Rust tests, adversarial security fixtures, and an FFI
  conformance harness.
- **Long-term extensible**: versioned manifest schema, versioned host API,
  versioned container format, provider seam, and a documented host API
  extension mechanism.

### 1.2 Non-negotiable rules

- Rust is the **only** component that parses archives or enforces
  path/package security. C++ never parses archives and never enforces security
  rules.
- No raw Rust references cross the FFI boundary. Opaque handles only.
- No panics cross the ABI. Every exported function catches unwind.
- Addons never see host filesystem paths. Errors never leak internal paths or
  secrets.
- No cross-addon contamination: each addon gets an isolated Lua state.

---

## 2. Layered Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Qt / C++ Application                        │
│                                                                     │
│  ┌────────────────────┐   ┌──────────────────────────────────────┐ │
│  │  AddonsDialog /    │   │  AddonHost (Lua runtime owner)      │ │
│  │  AddonMenu (UI)    │   │  - per-addon lua_State              │ │
│  └─────────┬──────────┘   │  - sandbox (libs, budgets, hooks)   │ │
│            │ signals      │  - lz.* host API (C functions)      │ │
│  ┌─────────▼──────────┐   │  - event dispatch to handlers       │ │
│  │ AddonManagerBridge │   │  - QIcon assembly from asset bytes  │ │
│  │  (FFI wrapper,     │   └──────────────────┬───────────────────┘ │
│  │   RAII handles)    │                      │ loads entry script  │
│  └─────────┬──────────┘                      │ bytes (no paths)    │
└────────────┼─────────────────────────────────┼─────────────────────┘
             │ C ABI (addons.h)                │
┌────────────▼─────────────────────────────────▼─────────────────────┐
│                Rust crate: lazydesktop-addons                     │
│                                                                     │
│  ffi.rs ─── (catch_unwind, null checks, handle mgmt, JSON)          │
│  registry.rs ── snapshot index (copy-on-write)                      │
│  provider/  ── AddonProvider trait                                 │
│                ├─ local.rs   (LocalProvider: dirs, zip, lzd)        │
│                └─ remote.rs  (RemoteProvider stub, design-ready)    │
│  package.rs ── AddonDescriptor, LoadedPackage, AssetReader          │
│  archive.rs ── zip/lzd parsing, limits, extraction                  │
│  manifest.rs ── TOML schema + validation (semver)                   │
│  ignore.rs  ── .lzdignore parser + gitignore matcher                │
│  pathsec.rs ── normalization, traversal checks, containment         │
│  error.rs   ── AddonError taxonomy + sanitized JSON                 │
└─────────────────────────────────────────────────────────────────────┘
             │ validated bytes only (no paths to UI)
┌────────────▼─────────────────────────────────────────────────────────┐
│                     Addon packages (untrusted)                      │
│   ~/.config/lazydesktop/addons/   user addons  (.zip/.lzd/dir)     │
│   /usr/share/lazydesktop/addons/  system addons (read-only)         │
└─────────────────────────────────────────────────────────────────────┘
```

**Data flow for a menu command contributed by an addon:**

```
User clicks Addon menu item
  → Qt signal → AddonManagerBridge (C++) → AddonHost
  → lua_pcall(handler) inside the addon's sandboxed lua_State
  → handler calls lz.ui.notify(...) → C host function (validates args)
  → Qt notification shown on main thread
```

Every layer talks only to its immediate neighbor. No layer reaches across.

---

## 3. Separation of Concerns

| Concern | Owner | Explicitly forbidden elsewhere |
|---|---|---|
| Archive parsing (zip/lzd) | Rust (`archive.rs`) | C++ must not open/inspect packages |
| Directory package loading | Rust (`provider/local.rs`) | — |
| `.lzdignore` parsing/matching | Rust (`ignore.rs`) | — |
| TOML parsing + schema validation | Rust (`manifest.rs`) | — |
| Path security (traversal, symlinks, containment) | Rust (`pathsec.rs`) | C++ must not re-validate paths |
| Resource access (asset bytes) | Rust (`package.rs` via FFI) | UI never reads package files directly |
| FFI boundary + error mapping | Rust (`ffi.rs`) | — |
| Lua state lifecycle (create/run/destroy) | C++ (`addon_host`) | Rust never runs Lua in v1 |
| Sandbox enforcement (libs, budgets) | C++ (`addon_host`) | Rust does not implement the Lua sandbox |
| Host API `lz.*` | C++ (`addon_host`, C functions) | — |
| QIcon conversion from validated bytes | C++ (`addon_manager_bridge`) | — |
| UI integration (menu, dialog, settings) | C++ (`mainwindow`, `addons_dialog`) | — |
| Enablement state (QSettings) | C++ | Rust tracks package validity only |

**Invariant:** the C++ layer *consumes* validated data (bytes, JSON strings,
enum codes). It never *validates* security-relevant data.

---

## 4. Repository Layout

The new Rust crate joins the existing Cargo workspace (`Cargo.toml` members)
and the XMake build alongside `ai_core` and `vcs_core`.

```
crates/addons/
├── Cargo.toml                 # crate-type = ["staticlib"], workspace member
├── addons.h                   # C ABI header (house style, see §11)
├── src/
│   ├── lib.rs                 # crate root, module wiring, #![forbid(unsafe_code)]
│   ├── error.rs               # AddonError taxonomy + sanitized JSON (§12)
│   ├── manifest.rs            # TOML schema, validation, semver (§6)
│   ├── ignore.rs              # .lzdignore parse + match (§7)
│   ├── archive.rs             # zip/lzd parse, limits, streaming extract (§8.3)
│   ├── package.rs             # AddonDescriptor, LoadedPackage, AssetReader
│   ├── pathsec.rs             # normalization, traversal, containment (§8.2)
│   ├── provider/
│   │   ├── mod.rs             # AddonProvider trait + ProviderError (§10)
│   │   ├── local.rs           # LocalProvider
│   │   └── remote.rs          # RemoteProvider (stub, returns NotSupported)
│   ├── registry.rs            # AddonRegistry + snapshot index
│   ├── json.rs                # serde structs for FFI JSON payloads
│   └── ffi.rs                 # C ABI exports (the ONLY unsafe module)
└── tests/
    ├── ffi_smoke.c            # C conformance harness (CI)
    ├── integration.rs         # registry/provider integration tests
    ├── security.rs            # adversarial fixtures (bombs, traversal, …)
    └── fixtures/              # valid + malicious sample packages (§20)
```

New C++/Qt files (following the existing bridge pattern):

```
src/addon_manager_bridge.h/.cpp   # RAII FFI wrapper, Qt signals, asset→QIcon
src/addon_host.h/.cpp             # Lua sandbox runtime, event dispatch, lz.* API
src/addons_dialog.h/.cpp          # Addons manager UI (list, enable, install, uninstall)
```

Lua runtime: **PUC Lua 5.4** (5.4.7 or newer patch). Obtained via the XMake
package manager (`add_requires("lua")`) on supported platforms, with a
documented fallback to a vendored amalgamation under `third_party/lua/`.
LuaJIT is explicitly **not** used (different semantics, weaker sandboxing
story, no `luaL_loadbufferx` guarantees).

---

## 5. Addon Package Format

### 5.1 Supported package kinds

| Kind | Detection | Handling |
|---|---|---|
| `.zip` | extension + PK magic | parsed and extracted by `archive.rs` with limits (§8.3) |
| `.lzd` | magic header (§5.3) | container parsed by `archive.rs`; payload is a zip |
| raw directory | contains `config.toml` | walked by `LocalProvider` with `follow_links(false)` |

### 5.2 Canonical layout

```
addon/
├── config.toml          # REQUIRED manifest (§6)
├── src/main.lua         # REQUIRED entry script (path declared in manifest)
├── assets/
│   ├── 16.ico           # optional icons (or .png)
│   ├── 32.ico
│   ├── 64.ico
│   └── 128.ico          # recommended minimum for UI integration
└── .lzdignore           # optional ignore file (§7)
```

- All paths inside a package use `/` separators in the manifest and archive.
- The entry script is loaded **by bytes** through the FFI; the host never
  resolves the entry path itself (§11, `lda_addon_entry_script`).

### 5.3 `.lzd` container format (version 1)

`.lzd` is a small, versioned container whose payload is a standard zip
archive. It is designed to carry a signature block in a later version without
changing the layout.

```
offset  size  field
0       8     magic      = b"LZDPKG01"            (ASCII)
8       4     version    u32 LE, must be 1
12      4     flags      u32 LE, bit 0 = signed   (v1: must be 0)
16      8     reserved   zero-filled (must be 0)
24      ...   payload    zip archive (same rules as .zip)
```

- v1: `flags & 1` must be `0`; a signed container is rejected with
  `InvalidAddon("signed packages are not supported by this version")` — an
  explicit refusal, never a silent ignore.
- The zip payload obeys every rule in §8.3.
- Trailing data after the zip payload is rejected.

---

## 6. Configuration Schema (config.toml)

Parsed by Rust with the `toml` crate. Unknown fields are tolerated (forward
compatibility) but never influence behavior; known fields are validated
strictly.

### 6.1 Field table

| Field | Type | Required | Rules |
|---|---|---|---|
| `id` | string | **yes** | `^[a-z0-9]+(\.[a-z0-9]+)+$` (reverse-DNS, >= 2 segments), <= 128 chars |
| `name` | string | **yes** | trimmed, 1..=128 chars |
| `version` | string | **yes** | strict semver 2.0.0 (`semver` crate) |
| `entry` | string | **yes** | relative POSIX path, must end `.lua`, must pass §8.2 traversal checks, <= 1024 chars |
| `description` | string | no | <= 512 chars |
| `author` | string | no | <= 128 chars |
| `license` | string | no | SPDX-style short id or "custom", <= 64 chars |
| `min_app_version` | string | no | semver; rejected if `min_app_version` > host app version |
| `max_app_version` | string | no | semver; rejected if `max_app_version` < host app version |
| `api_version` | string | no | `MAJOR.MINOR`, default `"1.0"`; addon `api_version.major` must equal host API major; `api_version.minor` must be <= host API minor |
| `schema_version` | integer | no | default `1`; values > supported schema -> `InvalidManifest` |

### 6.2 Validation rules (ordered, deterministic)

1. TOML parses, else `InvalidManifest`.
2. `schema_version` is supported, else `InvalidManifest`.
3. All required fields are present and type-correct, else `InvalidManifest`
   naming the offending `field`.
4. `id` matches the id regex, else `InvalidManifest` (`field: "id"`).
5. `version` parses as strict semver, else `InvalidManifest`.
6. `entry` is a valid relative path (no absolute, no `..`, no NUL, no
   backslash, ends `.lua`), else `InvalidManifest` / `PathSecurityViolation`.
7. Range checks vs host app version and host API version, else
   `InvalidManifest`.
8. Entry file exists in the package, is a regular file, is not excluded by
   ignore rules, else `InvalidAddon`.
9. Package has no other content errors (see §8.4), else the corresponding
   error.

### 6.3 Example (normative)

```toml
schema_version = 1
id = "example.addon"
name = "Example Addon"
version = "1.0.0"
entry = "src/main.lua"
description = "Demonstrates the addon contract."
author = "LazyDesktop Team"
license = "MIT"
api_version = "1.0"
min_app_version = "0.3.0"
```

### 6.4 Versioning and extensibility

- `schema_version` bumps only on breaking manifest changes. The registry
  rejects higher-than-supported values; the error names the supported max.
- Host API version (`lz.version()`) is independent: additive additions bump
  minor; breaking changes bump major and require addon `api_version`
  realignment.
- Future fields (e.g. `[permissions]`) are additive and introduced under the
  same `schema_version` when non-breaking, or a new `schema_version`
  otherwise.

---

## 7. Ignore System (.lzdignore)

### 7.1 Semantics

- Gitignore-style, one pattern per line.
- `#` begins a comment. Blank lines are ignored.
- `!` prefix re-includes (negation).
- Last matching pattern wins (gitignore precedence).
- A pattern with a trailing `/` matches directories only.
- `**` matches across directory boundaries; `*` does not match `/`.
- A pattern with a `/` anywhere (other than a trailing slash) is anchored to
  the package root; otherwise it matches at any level (gitignore rule).
- Per gitignore semantics, a file cannot be re-included if a parent directory
  is excluded.
- Pattern syntax, length limits: each line <= 512 bytes, total file <= 64 KiB.

### 7.2 Built-in default ignores (applied before user patterns)

```
.git/
.hg/
.svn/
.DS_Store
.lzdignore
```

The manifest `config.toml` is **never** ignorable: it is always parsed. If the
user pattern set would exclude it, extraction still keeps it (manifest is
metadata, not content). This is the single documented exception.

### 7.3 Where ignore rules apply

| Operation | Behavior |
|---|---|
| Archive extraction | Excluded entries are dropped at extraction time (never written to disk) |
| Asset reads (`lz.assets.read`) | Requested path must not be ignored; `config.toml` and `.lzdignore` are additionally denied |
| Entry script validation | Entry must not be ignored, else `InvalidAddon` |
| Content accounting (size, entry counts) | Ignored entries are not counted |

### 7.4 Implementer's notes

- Matching operates on normalized, `/`-separated relative paths (see §8.2).
- Implemented in `ignore.rs` with a dedicated `IgnoreMatcher` struct.
- The matcher is applied **after** path normalization and traversal rejection:
  ignore rules never weaken path security; they only filter content.

---

## 8. Security Model

Rust is the trust boundary. Everything below is enforced by the crate; C++
never re-derives any of it.

### 8.1 Threat model

Threats defended against (each with its defense):

| Threat | Defense |
|---|---|
| Path traversal (zip-slip, `..`, absolute) | §8.2 normalization + containment |
| Symlink escape | §8.2 symlink policy |
| Archive bomb (size/ratio/entry explosion) | §8.3 hard limits, streaming |
| Duplicate/overlapping entries | §8.3 reject duplicates |
| Encrypted / malformed archives | §8.3 reject encrypted; CRC + structural validation |
| Hostile manifest (wrong types, huge strings, bad version) | §6 validation, size caps |
| Ignore bypass (`!.lzdignore`, weird encodings) | §7 fixed rules, `config.toml` exception |
| Lua escapes (io/os/load/debug access, memory/CPU exhaustion) | §13 sandbox, budgets, hooks |
| Error introspection (leaked host paths/secrets) | §12 sanitization |
| FFI misuse (nulls, double-free, allocator mismatch, panics) | §11 ABI rules |

### 8.2 Path security (`pathsec.rs`)

**Normalization.** Every path inside a package (archive entry name, manifest
`entry`, asset request, ignore pattern) is first normalized to a canonical
relative form:

- Reject NUL bytes anywhere.
- Reject empty paths and paths that normalize to the root.
- Reject absolute paths (leading `/`, Windows drive letters `C:`).
- Reject any path component equal to `.` or `..`.
- Reject backslash as a path separator on all platforms (in archive entries it
  is treated as an attack vector, not a separator).
- Reject paths longer than 1024 bytes (UTF-8).
- Normalize `/` separators; strip a single leading `./` and any duplicated
  slashes.

**Containment.** After extraction or during directory walking, every resolved
file is checked with the filesystem itself:

1. `fs::canonicalize` the parent directory of the target.
2. Verify the canonical parent path starts with the canonical package root.
3. This closes TOCTOU and symlink races at read time, in addition to the
   lexical checks.

**Symlink policy.**

- Archives: symlink entries are extracted **only** if the link target is
  relative and, after joining with the entry's directory, resolves lexically
  inside the package root. Symlinks that point outside are rejected with
  `PathSecurityViolation`. Extraction never follows a symlink entry.
- Directory packages: walking uses `follow_links(false)`. Symlinked files
  inside the package are usable only when their canonical target stays within
  the package root (checked at read time).
- Asset reads and entry-script reads resolve through the containment check;
  a symlink that escapes the root yields `PathSecurityViolation`.

**Storage paths.** Host-side per-addon storage paths are never derived from
addon input. The crate exposes `lda_addon_storage_dir` (see §11), which
returns the host-computed storage directory for an addon id after validating
the id. Lua never receives the path; it only uses `lz.storage.*`.

### 8.3 Archive protection (`archive.rs`)

Hard limits (defaults; provider-configurable within a bounded range):

| Limit | Default |
|---|---|
| Max archive file size (compressed) | 1 GiB |
| Max entries | 4096 |
| Max total uncompressed size | 512 MiB |
| Max single entry uncompressed | 128 MiB |
| Max compression ratio (streamed, rolling) | 300:1 |
| Max entry name length | 1024 bytes |

Enforcement:

- **Streaming extraction** with a running byte budget. The moment any limit is
  exceeded the extraction aborts and the partially extracted directory is
  deleted. No archive is ever materialized fully in memory.
- Duplicate entry names are rejected (first-wins is not permitted).
- Encrypted entries are rejected (zip encryption unsupported in v1).
- CRC32 is verified per entry by the zip reader; a mismatch is `ArchiveError`.
- Directory entries are validated (trailing `/`); directories are created
  strictly from validated entry paths.
- Entry file permissions from the archive are ignored; extracted files are
  created `0600`, directories `0700`. No extracted file is ever executable.
- Zip timestamps are ignored; extraction does not preserve mtime.
- Nested archives are never extracted (flat addon model).
- After extraction, the crate re-scans the extracted tree to confirm the entry
  script and assets exist and are regular files (defense in depth).

### 8.4 Package content rules (all kinds)

- Exactly one manifest (`config.toml`) is required.
- The entry script must exist, be a regular file, be within the root, and not
  be ignored (§6.2 rule 8).
- Icons (`assets/16|32|64|128.ico|.png`) are optional; at least one size is
  recommended. Icon bytes are never interpreted by Rust — they are returned to
  C++ which decodes them with Qt in a failure-tolerant way (fallback icon on
  decode failure). Dimension checks are not performed in v1.

### 8.5 `.lzdignore` safety

- Ignore matching never grants access; it only removes content from
  consideration.
- The manifest exception is the only hard-coded special case.
- Patterns are applied to normalized relative paths only.

---

## 9. Rust Core Design

### 9.1 Crate metadata

- Name: `lazydesktop-addons`; directory `crates/addons`.
- `crate-type = ["staticlib"]`; edition 2021; joins the workspace root
  `Cargo.toml` members. Release profile inherits workspace `lto = true`,
  `opt-level = "z"`, `strip = true`.
- `#![forbid(unsafe_code)]` on `lib.rs`; the single exception module `ffi.rs`
  opts out with a narrow `#![allow(unsafe_code)]` and contains **all**
  `unsafe` blocks in the crate. A CI grep asserts no other `unsafe` remains.

### 9.2 Dependencies

| Crate | Purpose |
|---|---|
| `zip` (default-features off: `deflate`, `bzip2` optional off) | zip parsing with streaming extraction; exposes entry metadata and CRC |
| `toml` | manifest parsing |
| `serde` + `serde_json` | typed manifest + FFI JSON payloads |
| `semver` | strict version parsing and range checks |
| `thiserror` | error taxonomy with `From` conversions |
| `walkdir` | directory package walking (`follow_links(false)`) |
| `tempfile` (dev) | hermetic test fixtures |
| `proptest` (dev) | property tests for ignore matching and path normalization |

No async runtime. All crate operations are synchronous and bounded.

### 9.3 Core types (`package.rs`)

```rust
pub struct AddonId(String);              // validated reverse-DNS form

pub struct AddonDescriptor {
    pub id: AddonId,
    pub name: String,
    pub version: semver::Version,
    pub entry: String,                   // normalized relative path
    pub description: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
    pub api_version: (u32, u32),         // major, minor
    pub provider: String,                // "local", ...
    pub root_kind: RootKind,             // Directory | Zip | Lzd
    pub size_bytes: u64,
    pub has_icons: BTreeMap<u32, bool>,  // {16: .., 32: .., 64: .., 128: ..}
}

pub enum PackageSource {                 // input to install/load
    Zip { path: PathBuf },
    Lzd { path: PathBuf },
    Directory { path: PathBuf },
}

pub struct LoadedPackage {               // an opened, validated package
    id: AddonId,
    root: PathBuf,                       // canonical, validated at open
    ignore: IgnoreMatcher,
    manifest: AddonManifest,
    // guarded by the registry's internal locking; handed out as Arc
}

impl LoadedPackage {
    pub fn read_asset(&self, rel: &str) -> Result<Vec<u8>, AddonError>;
    pub fn entry_script(&self) -> Result<Vec<u8>, AddonError>;
}
```

`LoadedPackage::read_asset` applies: path normalization (§8.2) → traversal
rejection → ignore check (§7.3) → denial of `config.toml`/`.lzdignore` →
canonical containment check (§8.2) → bounded read (<= 64 MiB per asset).

### 9.4 Determinism guarantees

- Discovery order: user root before system root; within a root, ids sorted
  lexicographically (byte order, UTF-8).
- Error selection is deterministic: first error encountered during the
  ordered validation pipeline (§6.2) is reported.
- No wall-clock dependence anywhere except the host-side Lua watchdog (§13.4).
- Archive extraction is idempotent with respect to the content limits; entry
  order is sorted before extraction.

---

## 10. Provider Architecture

### 10.1 Trait

```rust
pub trait AddonProvider: Send + Sync {
    fn provider_id(&self) -> &str;
    fn display_name(&self) -> String;

    /// Scan all sources known to this provider; return validated descriptors.
    fn load(&self) -> Result<Vec<AddonDescriptor>, ProviderError>;

    /// Install a package source; returns the installed descriptor.
    fn install(&self, source: &PackageSource) -> Result<AddonDescriptor, ProviderError>;

    /// Remove an installed addon (user providers only; system providers
    /// return ProviderError::ReadOnly for uninstall).
    fn uninstall(&self, id: &AddonId) -> Result<(), ProviderError>;

    /// Open an installed addon for execution/asset access.
    fn open(&self, id: &AddonId) -> Result<Arc<LoadedPackage>, ProviderError>;

    fn is_read_only(&self) -> bool { false }
}
```

`ProviderError` maps into the shared taxonomy (§12) via `AddonError::Provider`.

### 10.2 LocalProvider (`provider/local.rs`)

- Roots: user addon directory and system addon directory (host-provided
  paths; e.g. `~/.config/lazydesktop/addons` and
  `/usr/share/lazydesktop/addons`, platform variants for Windows/macOS).
- Scanning per root:
  1. Walk entries sorted by name.
  2. Subdirectories containing `config.toml` → `PackageSource::Directory`
     (loaded in place).
  3. Files ending `.zip` → archive source → validated → extracted into the
     provider cache (`addons-cache/<hash>/`, hash of (path, mtime, size)) →
     loaded from cache.
  4. Files ending `.lzd` → container parse → same extraction path.
- On duplicate `id` across roots: user root wins; the system duplicate is
  still listed with `provider` + `state: "shadowed"` so the UI can surface it.
- On duplicate `id` within a root: higher `version` wins; the loser is listed
  as `state: "shadowed"`. Equal versions → `ProviderError::Conflict` at
  load time (surfaced in the addons dialog, not fatal to other addons).
- `uninstall` removes the user-root directory/archive and its cache entry.
  System addons are read-only; disablement is handled by C++ enablement state.

### 10.3 RemoteProvider (`provider/remote.rs`)

- v1: the trait is implemented but every operation returns
  `ProviderError::NotSupported` (explicit, testable failure — no silent
  no-op).
- Design intent (documented, not built in v1): registry index fetch → signed
  package download (ed25519, key pinned in the binary) → cryptographic hash
  verification → cache into the provider cache directory → delegate to the
  same `LoadedPackage` machinery as LocalProvider. No UI changes are needed
  because the UI consumes only descriptors through the registry.

### 10.4 Registry (`registry.rs`)

- `AddonRegistry` holds `Vec<Arc<dyn AddonProvider>>` and a snapshot index:
  `Arc<RegistrySnapshot>` where `RegistrySnapshot { by_id: HashMap<AddonId,
  AddonEntry> }`.
- `load()`/`refresh()` rebuild the snapshot atomically (copy-on-write); readers
  grab the `Arc` and never block on writers.
- Operations: `list()`, `get(id)`, `install(source)`, `uninstall(id)`,
  `open(id)`, `read_asset(id, rel)`, `entry_script(id)`, `storage_dir(id)`.
- All methods return `Result<_, AddonError>`; no panics escape (they are
  caught in `ffi.rs` anyway, but internal code is total).

---

## 11. FFI ABI Design

### 11.1 Conventions (house style, matching `vcs_core.h` / `ai_core.h`)

- C header `crates/addons/addons.h` with `extern "C"` guards, `stdbool.h`,
  `stddef.h`, `stdint.h`.
- Opaque handle typedefs only (`lda_registry`).
- `char *` returns are heap-allocated UTF-8 and MUST be freed with
  `lda_free_string` (never `free()`).
- Byte buffers are returned through out-params `uint8_t **out, size_t *out_len`
  and MUST be freed with `lda_free_bytes`.
- Every export is wrapped in `panic::catch_unwind(AssertUnwindSafe(..))`;
  a caught panic returns `LDA_ERR_FFI` with a generic message (no panic
  payload leaks).
- Every pointer argument is null-checked; nulls return
  `LDA_ERR_INVALID_ARGUMENT`.
- Ownership is always explicit: `*_create`/`*_destroy` pairs; destroy takes
  `lda_registry **` and nulls the caller's handle.
- Versioned ABI: `lda_api_version(uint32_t *major, uint32_t *minor)` plus
  `#define LDA_ABI_VERSION 1` in the header.
- **No raw Rust references ever cross the boundary.** Handles are
  `NonNull<Arc<..>>`-style opaque pointers; the callee never dereferences
  caller-provided pointers except by calling back into lda API.

### 11.2 Result codes

```c
typedef enum lda_result {
    LDA_OK = 0,
    LDA_ERR_INVALID_ARGUMENT,   // null/type errors from the C caller
    LDA_ERR_INVALID_ADDON,      // package-level validity failure
    LDA_ERR_INVALID_MANIFEST,   // config.toml schema/validation failure
    LDA_ERR_ARCHIVE,            // zip/lzd structure, CRC, limits
    LDA_ERR_PATH_SECURITY,      // traversal/symlink/containment violation
    LDA_ERR_PROVIDER,           // provider-level failure (conflict, read-only)
    LDA_ERR_LUA_RUNTIME,        // produced by the C++ host, shared taxonomy
    LDA_ERR_FFI,                // internal panic caught at the boundary
    LDA_ERR_INTERNAL,           // unexpected internal error (io, alloc)
} lda_result;
```

Structured details are returned as JSON (§12) through out-params. The enum is
for fast dispatch and tests; the JSON is the user-facing contract.

### 11.3 Function surface

```c
/* Version */
void lda_api_version(uint32_t *major, uint32_t *minor);
const char *lda_abi_version(void);              // "1"

/* Memory */
void lda_free_string(char *s);
void lda_free_bytes(uint8_t *p);

/* Logging sink (optional; see §15 for callback rules) */
typedef void (*lda_log_fn)(int level, const char *msg, void *userdata);
void lda_set_log_sink(lda_log_fn cb, void *userdata);

/* Registry lifecycle */
lda_registry *lda_registry_create(void);                    // never NULL on success
lda_result lda_registry_destroy(lda_registry **r);          // nulls *r
lda_result lda_registry_set_app_version(lda_registry *r, const char *version);
lda_result lda_registry_add_root(lda_registry *r, const char *path, int user_priority);
lda_result lda_registry_load(lda_registry *r);
lda_result lda_registry_refresh(lda_registry *r);

/* Listing & lookup */
lda_result lda_registry_list(lda_registry *r, char **out_json);     // JSON array of descriptors
lda_result lda_registry_get(lda_registry *r, const char *id, char **out_json);

/* Install / uninstall */
lda_result lda_registry_install(lda_registry *r, const char *source_path, char **out_json);
lda_result lda_registry_uninstall(lda_registry *r, const char *id);

/* Resource access (validated bytes only) */
lda_result lda_addon_has_asset(lda_registry *r, const char *id, const char *rel_path, int *out_has);
lda_result lda_addon_read_asset(lda_registry *r, const char *id, const char *rel_path,
                                uint8_t **out_bytes, size_t *out_len);
lda_result lda_addon_entry_script(lda_registry *r, const char *id,
                                  uint8_t **out_bytes, size_t *out_len);
lda_result lda_addon_storage_dir(lda_registry *r, const char *id, char **out_path);
```

### 11.4 Ownership table

| Returned | Freed by | Notes |
|---|---|---|
| `char *` (JSON, paths to C++) | caller via `lda_free_string` | never `free()` |
| `uint8_t *` byte buffers | caller via `lda_free_bytes` | Rust global allocator |
| `lda_registry *` handle | `lda_registry_destroy(&r)` exactly once | double destroy is UB; the destroy function nulls the handle |
| `out` scalars / flags | caller stack | — |

### 11.5 FFI safety rules (complete list)

1. No raw Rust references over the boundary — handles only.
2. No panics across the ABI — `catch_unwind` on every export.
3. Opaque handles only; internal layout never exposed.
4. Explicit ownership and destroy functions; destroy nulls the handle.
5. No allocator mismatches — Rust frees what Rust allocates (`lda_free_*`).
6. Null safety required — every pointer argument checked.
7. Strings are UTF-8; invalid UTF-8 from the caller is
   `LDA_ERR_INVALID_ARGUMENT` (never UB).
8. Callback (`lda_log_fn`) rules: invoked on the calling thread; must not
   re-enter the lda API; must not block; must not use the Rust allocator.
9. Functions are re-entrant and thread-safe (registry is internally locked);
   the crate never returns borrowed data.
10. ABI versioning: any breaking change bumps `LDA_ABI_VERSION`; the C++ side
    checks `lda_api_version` at startup and refuses to run mismatched builds.

---

## 12. Error Model

### 12.1 Taxonomy

```rust
pub enum AddonError {
    InvalidAddon { id: Option<AddonId>, message: String },
    InvalidManifest { id: Option<AddonId>, field: Option<String>, message: String },
    ArchiveError { source: String, message: String },
    PathSecurityViolation { path: String, message: String },   // path is relative
    LuaRuntimeError { addon_id: AddonId, message: String },    // produced by host
    ProviderError { provider: String, message: String },
    FfiError { message: String },
    Internal { message: String },                               // io, alloc, unexpected
}
```

- `PathSecurityViolation.path` is the **normalized relative** path (never an
  absolute host path).
- `message` strings are written by the crate to be safe for display; they
  never contain absolute paths, environment variables, or secrets. A
  sanitization pass (see below) runs before JSON serialization as a backstop.

### 12.2 JSON shape (FFI `out_json`)

Success (list/get/install):

```json
{ "addons": [ { "id": "example.addon", "name": "Example Addon",
                "version": "1.0.0", "entry": "src/main.lua",
                "description": "...", "author": "...", "license": "MIT",
                "api_version": [1, 0], "provider": "local",
                "root_kind": "directory", "size_bytes": 1234,
                "has_icons": { "16": true, "32": true, "64": true, "128": true },
                "state": "installed" } ] }
```

Single object forms for `get`/`install`.

Failure:

```json
{ "error": { "code": "InvalidManifest", "message": "field 'version' is not valid semver",
             "addon_id": "example.addon", "field": "version" } }
```

### 12.3 Sanitization rules

- Absolute paths are rewritten to `<internal>`.
- Environment variable values (`$HOME`, API keys, tokens) are never included;
  the crate only emits enum codes and purpose-written messages.
- `%` and control characters in messages are escaped for display.
- The sanitizer is unit-tested with adversarial message fixtures.

### 12.4 Error → result-code mapping

| Error | `lda_result` |
|---|---|
| `InvalidManifest` | `LDA_ERR_INVALID_MANIFEST` |
| `InvalidAddon` | `LDA_ERR_INVALID_ADDON` |
| `ArchiveError` | `LDA_ERR_ARCHIVE` |
| `PathSecurityViolation` | `LDA_ERR_PATH_SECURITY` |
| `ProviderError` | `LDA_ERR_PROVIDER` |
| `LuaRuntimeError` | `LDA_ERR_LUA_RUNTIME` |
| `FfiError` / caught panic | `LDA_ERR_FFI` |
| `Internal` | `LDA_ERR_INTERNAL` |
| Null/invalid C args | `LDA_ERR_INVALID_ARGUMENT` |

---

## 13. Lua Execution Model

Owned and implemented entirely by the C++ host (`addon_host`). Rust never
executes Lua in v1; Rust's role is limited to delivering validated entry-script
bytes.

### 13.1 Runtime model

- One isolated `lua_State` per addon, created at addon init and destroyed at
  addon teardown. No state is shared between addons.
- All Lua execution happens on the Qt main thread (§15). Scripts are
  short-lived; runaway scripts are killed by the instruction hook and the
  memory allocator budget.
- The entry script is loaded from validated bytes with
  `luaL_loadbufferx(L, bytes, len, chunkname, "t")` using chunkname
  `=addon:<id>` (the `=` prefix keeps the id out of line numbers; the id is
  sanitized by the manifest rules). The host never passes a filesystem path to
  Lua.
- After load, the chunk's `_ENV` upvalue is replaced with the sandboxed
  environment table, then executed with `lua_pcall`.

### 13.2 Sandbox boundaries

**Restricted library set.** Only these libraries are opened (via selective
opening — never `luaL_openlibs` wholesale):

| Library | Status | Notes |
|---|---|---|
| `base` | **stripped** | keeps `print` → routed to addon log, `type`, `pairs`, `ipairs`, `select`, `error`, `assert`, `tostring`, `tonumber`, `rawget/rawset/rawlen`, `next`, `pcall`, `xpcall`, `coroutine` is separate; **removes** `load`, `loadfile`, `dofile`, `require`, `collectgarbage`, `getmetatable`/`setmetatable` (see hardening) |
| `string` | full | except `string.dump` is removed (blocks bytecode smuggling) |
| `table` | full | |
| `math` | full | |
| `utf8` | full | |
| `coroutine` | full | instruction budget applies per Lua thread (see §13.3) |
| `io`, `os`, `debug`, `package` | **absent** | never opened; `require` does not exist |

**Hardening passes after setup:**

1. Remove `load`, `loadfile`, `dofile`, `require`, `collectgarbage`,
   `getmetatable`, `setmetatable`, `string.dump` from the environment.
2. Set a metatable on the sandbox `_G` with `__newindex` raising an error:
   addons cannot create new global names after init (deterministic, prevents
   env tampering).
3. Set a metatable on the global environment with `__metatable` returning a
   fixed dummy value so `getmetatable` cannot inspect it even if it were
   reachable.
4. No `debug` library ⇒ no `debug.upvalueid/upvaluejoin`, no `getinfo` escape.
5. No `package.cpath`/`cpath` ⇒ no native library loading.
6. The host API table `lz` is a frozen table of C functions (metatable
   `__newindex` errors; `__metatable` guarded).

**Host API exposure.** A single global `lz` table. v1 surface (all C
functions, all arguments validated with `luaL_check*`):

```lua
lz.version() -> { major = 1, minor = 0 }       -- host API version
lz.log(level, message)                          -- level: "trace"|"debug"|"info"|"warn"|"error"
lz.get_context() -> table                       -- { repo_path?, branch?, app_version, provider }
lz.assets.has(rel_path) -> boolean
lz.assets.read(rel_path) -> string | nil, err   -- validated bytes, decoded UTF-8
lz.ui.notify(title, message, kind)              -- kind: "info"|"warning"|"error"
lz.ui.add_command({ id, label, handler })       -- register a menu command (see §16.2)
lz.storage.get(key) -> string | nil
lz.storage.set(key, value)                      -- value is a plain string
```

Rules for the host API:

- All I/O is mediated: filesystem only via `lz.assets.*` (Rust-validated) and
  `lz.storage.*` (host-owned, id-namespaced file); network is **not** exposed
  in v1.
- Host passes only plain data (strings, numbers, booleans, tables) to Lua.
  No userdata, no pointers, no lightuserdata (the per-addon context lookup key
  is a lightuserdata held only in the registry and never exposed).
- Every C host function is re-entrant and must not call back into the same
  addon's Lua state (no reentrancy loops).

### 13.3 Budgets

| Budget | Enforcement | Default |
|---|---|---|
| Memory | `lua_newstate` with a custom `lua_Alloc` wrapper that counts bytes and aborts allocation over the cap | 64 MiB per addon state |
| Instructions | `lua_sethook` count hook on the addon state | 20,000,000 instructions per invocation; 200,000,000 total per addon run |
| Time | host-side Qt watchdog (single-shot timer) that tears down a stuck invocation | 5 s per invocation |

Budget exhaustion is reported as `LuaRuntimeError` with a sanitized message
(`"addon exceeded its instruction budget"`, `"addon exceeded its memory
budget"`, `"addon timed out"`). The addon state is destroyed after a hard
budget kill (a corrupted/broken addon is never kept running).

### 13.4 Error handling

- Entry script and every handler invocation run through `lua_pcall` with a
  message handler.
- Lua error strings may contain chunkname (`addon:<id>`), line numbers, and
  addon-controlled text. The host sanitizes error messages (strip anything
  that looks like an absolute path; cap length at 4 KiB) before mapping to
  `LuaRuntimeError`.
- Errors are reported through Qt signals to the UI (notification + log); they
  never crash the host and never affect other addons.
- An addon whose `on_ready` throws is marked `state: "error"` in the UI and is
  not offered for command execution until re-enabled (re-init on next launch).

### 13.5 Isolation guarantees

- Separate `lua_State` per addon ⇒ no shared globals, no cross-addon
  contamination by construction.
- Host API calls are bound to the calling addon's context (looked up from the
  state's registry, keyed per addon), so `lz.storage.*` and logging are
  automatically namespaced.
- Addons cannot reach other addons' states: no `debug`, no shared tables, no
  global registry access.

---

## 14. C++/Qt Host Design

### 14.1 `addon_manager_bridge` (FFI wrapper)

- RAII class owning the `lda_registry*`; destructor calls
  `lda_registry_destroy(&r)`.
- Startup: check `lda_api_version`, `lda_registry_create`, set app version,
  add roots (user + system paths resolved from `QStandardPaths`), `load`.
- Thin wrappers per FFI function with `QByteArray`/`QString` conversion and a
  `struct AddonErrorInfo { lda_result code; QString message; QString addonId;
  QString field; }` extracted from the JSON error object.
- Qt signals: `addonsLoaded`, `installFinished(descriptorJson, error)`,
  `uninstallFinished(id, error)`, `refreshFinished(error)`.
- `QIcon` assembly: `lda_addon_read_asset(id, "assets/128.ico", …)` → bytes →
  `QPixmap::loadFromData(bytes, "ICO")` (Qt ships an ICO reader); fallback to
  `"PNG"`; fallback to a built-in placeholder icon. Sizes 16/32/64/128
  attempted in order; failure is non-fatal.

### 14.2 `addon_host` (Lua runtime)

- Owns `std::unordered_map<AddonId, AddonRuntime>` where `AddonRuntime { lua_State*,
  context table, registered commands, storage cache, runtime budget state }`.
- `initializeAddon(id)`: fetch entry bytes via bridge → create sandboxed
  state (§13.2) → run entry → read returned table → call `on_ready(ctx)`.
- `invokeCommand(addonId, commandId)`: look up handler reference in the state
  registry, `lua_pcall` with instruction hook armed, watchdog timer started.
- `destroyAddon(id)`: call `on_destroy()` (best-effort, pcall), remove
  registered commands from the UI, `lua_close`, erase runtime.
- All `lua_State` creation uses the budgeted allocator and the hardened
  library setup described in §13; the sandbox setup code is shared and unit
  tested.
- Shutdown order (deterministic): UI menu teardown → all `destroyAddon` in id
  order → bridge destructor → registry destroy.

### 14.3 `addons_dialog` (UI)

- Lists installed addons (icon, name, version, provider, state), enable
  checkboxes, Install (file picker for `.zip`/`.lzd`), Uninstall, Refresh.
- Reads descriptors from the bridge's parsed JSON; enablement persisted in
  QSettings (§18).
- File picker filters `.zip`/`.lzd`; raw-directory install is offered through
  a folder picker (copy-into-user-root semantics handled by the bridge calling
  `lda_registry_install` with the directory path).

### 14.4 UI integration

- A top-level **Addons** menu is populated from all enabled addons'
  registered commands. Menu actions map `(addonId, commandId)` → host
  `invokeCommand`.
- Notifications via the existing system-tray/notification path.

---

## 15. Threading Model

### 15.1 Qt main thread

- Owns: all UI, `AddonManagerBridge`, `AddonHost`, every `lua_State`, and the
  addons dialog. All Lua execution happens here. This makes addon behavior
  deterministic and free of data races by construction.

### 15.2 Qt worker thread(s)

- Long operations — `load`, `refresh`, `install`, `uninstall` (filesystem and
  archive work) — run on a Qt worker (`QtConcurrent::run`), with results
  marshalled to the main thread via queued signals.
- Workers only touch the FFI bridge through thread-safe lda calls; they never
  touch Qt widgets or Lua.

### 15.3 Rust concurrency safety

- `AddonRegistry` is `Send + Sync`; internal state is guarded (snapshot index
  is copy-on-write `Arc`; provider internals use a `Mutex` where needed).
- FFI exports are re-entrant; a handle may be used from multiple threads
  concurrently.
- No `'static` borrows escape; everything crossing the ABI is owned data
  (bytes, JSON strings) or opaque `Arc` handles.

### 15.4 Provider thread safety

- `AddonProvider` implementations must be `Send + Sync`. `LocalProvider`
  serializes filesystem scans with an internal lock; its cache writes are
  atomic (write-temp-then-rename).
- The registry never holds a lock while calling user callbacks (none exist in
  v1) or while opening packages.

### 15.5 Callback rules (logging sink)

- `lda_log_fn` is invoked on the thread performing the lda operation; it must
  not re-enter lda, must not block, and must not use the Rust allocator. The
  C++ bridge forwards to `qDebug` on the main thread via a queued signal.

---

## 16. Lifecycle Models

### 16.1 Package lifecycle (registry, Rust)

```
discovered ──validate──> installed ──open──> loaded
   (found in root)      (extracted/valid)   (LoadedPackage in memory)
       │                    │  ▲
       │  version/conflict  │  └──── refresh() re-validates
       ▼                    ▼
  shadowed / error      uninstalled (user root only)
```

- `discovered`: raw source located but not yet validated.
- `installed`: manifest + content validated; descriptor in the snapshot.
- `loaded`: an `Arc<LoadedPackage>` is open for asset/entry access.
- `shadowed`: duplicate id overridden by a higher-priority or higher-version
  copy; still listed for transparency.
- `error`: validation failed; descriptor carries `state: "error"` and the UI
  surfaces the sanitized message.

### 16.2 Addon runtime lifecycle (host, C++)

```
create ──> init ──> ready ──> running ──> destroy
 (state)  (entry   (on_ready  (handlers   (on_destroy,
           script   registered  invoked    lua_close,
           + sandbox commands)  via menu)  UI cleanup)
```

1. **create**: fetch entry bytes (bridge → `lda_addon_entry_script`); create
   sandboxed `lua_State` with budgeted allocator; install restricted libs;
   set `_ENV`; register `lz` C API bound to this addon.
2. **init**: `luaL_loadbufferx` + `lua_pcall`. The entry executes once. If it
   returns a table, that table is the addon object; `on_ready(ctx)` and
   `on_destroy()` are read from it. No return value is allowed as well (the
   addon may register everything in `on_ready`); a thrown error → `state:
   "error"`.
3. **ready**: commands registered via `lz.ui.add_command` are published to the
   UI menu. Invocation of a command runs a fresh handler call.
4. **running**: each handler invocation gets its own instruction budget and
   watchdog window.
5. **destroy**: `on_destroy()` best-effort (pcall), commands removed, UI
   cleaned up, `lua_close`, state freed. Destroy is total: no signals are
   emitted during teardown of the same addon.

**Addon contract example (normative for addon authors):**

```lua
-- src/main.lua
local addon = {}

function addon.on_ready(ctx)
  lz.log("info", "example.addon ready")
  lz.ui.add_command({
    id = "greet",
    label = "Greet",
    handler = function()
      lz.ui.notify("Hello", "Hello from the example addon!", "info")
    end,
  })
end

function addon.on_destroy()
  lz.log("info", "example.addon destroyed")
end

return addon
```

---

## 17. Asset Handling Pipeline

```
Package on disk (untrusted)
  → Rust: normalize path → traversal check → ignore check → containment check
  → Rust: bounded read (≤ 64 MiB)
  → FFI: validated bytes (lda_addon_read_asset)
  → C++ bridge: QByteArray (copy), decode via Qt
  → QIcon / QPixmap for UI; Lua sees strings via lz.assets.read
```

- Rust validates **all** paths and performs **all** reads. C++ consumes only
  validated byte buffers.
- The UI never opens package files directly; there is no filesystem access to
  package internals from the Qt layer.
- Icon decoding is failure-tolerant: decode failure → next size → built-in
  fallback icon. Invalid image bytes never crash the host (Qt image plugins
  report load failure).
- The entry script is delivered the same way (`lda_addon_entry_script`),
  which is what keeps Lua and its error messages free of host paths.

---

## 18. Configuration and Settings Integration

### 18.1 Host settings (QSettings, C++)

- Enablement: `addons/<id>/enabled` (bool, default true).
- Install/refresh metadata is owned by Rust; enablement is owned by C++.
- The addons dialog merges Rust descriptors with enablement state.

### 18.2 Per-addon storage (host-owned, id-namespaced)

- Storage root: `~/.config/lazydesktop/addon-data/<id>/storage.json`.
- The directory path is computed by Rust (`lda_addon_storage_dir`, id
  validated) and given to C++; it is **never** exposed to Lua.
- Lua accesses storage only through `lz.storage.get/set` (plain string
  values; keys validated by the host against `^[a-zA-Z0-9._-]{1,128}$`).
- The C++ host serializes storage.json atomically (temp + rename) on the main
  thread; corruption is treated as empty storage (never a crash).
- Storage is destroyed when an addon is uninstalled (host-side cleanup after
  a successful `lda_registry_uninstall`).

### 18.3 Addon system settings (Rust)

- No persistent state in Rust in v1; every load is a fresh, deterministic
  scan. Limits (§8.3) use built-in defaults; a future settings surface can
  pass them through `lda_registry_set_limits` without changing the ABI shape.

---

## 19. Build Integration

### 19.1 Cargo workspace

- Add `"crates/addons"` to workspace members in the root `Cargo.toml`.
- The crate uses the workspace release profile (`lto`, `opt-z`, `strip`).

### 19.2 XMake

- New target wiring in `xmake.lua` mirroring `ai_core`/`vcs_core`:
  - `before_build`: `cargo build --lib --manifest-path crates/addons/Cargo.toml`
    (debug/release mirroring `is_mode`).
  - `add_includedirs("crates/addons")`, `add_links("addons")`.
  - The existing `--allow-multiple-definition` Linux/macOS linker flag already
    covers the third embedded Rust std.
- Lua: `add_requires("lua")` (XMake package, Lua 5.4) plus
  `add_packages("lua")`; documented fallback to a vendored amalgamation
  (`third_party/lua/lua.c` + headers) if the package is unavailable.

### 19.3 CI

- `ci.yml`: `cargo test --workspace` already runs; the new crate's tests are
  included automatically. Add a job step compiling and running
  `tests/ffi_smoke.c` against the built staticlib on each platform (Linux
  primary; compile-only smoke on Windows/macOS).
- The unsafe audit step: grep for `unsafe` outside `src/ffi.rs` (fails the
  build if found).

---

## 20. Testing Strategy

Priorities honored: memory safety and security behavior are tested first and
most aggressively; convenience features are not tested at all until the core
is green.

### 20.1 Rust unit tests (in-crate)

- `manifest.rs`: validation matrix — every rule in §6.2, including each
  required-field miss, regex rejections, semver edge cases, schema version
  rejection, api_version compatibility, and the ordered-error determinism
  guarantee.
- `ignore.rs`: pattern semantics — comments, negation, anchoring, trailing
  slash, `**`, parent-exclusion rule, and the `config.toml` exception.
- `pathsec.rs`: normalization rejections (absolute, drive letters, `..`, NUL,
  backslash, overlong), containment via canonicalization, symlink policy
  (in-root allowed, escaping rejected).
- `archive.rs`: limits — entry count, total size, single size, ratio, name
  length; duplicate entries; encrypted entries; CRC mismatch; trailing data
  after lzd payload; extraction cleanup on abort.
- `error.rs`: JSON shape, sanitization of absolute paths/control characters.

### 20.2 Rust integration tests (`tests/integration.rs`)

- Build fixture packages in `tempfile` dirs: directory, zip, lzd.
- Drive `AddonRegistry` + `LocalProvider` end-to-end: load → list → open →
  read asset → entry script → uninstall; user-over-system priority; version
  shadowing; conflict reporting.
- Verify deterministic ordering of list output.

### 20.3 Security fixtures (`tests/security.rs` + `tests/fixtures/`)

Adversarial packages checked at build time from fixture bytes:

- zip-slip (`../escape.txt`), absolute path entry, drive letter, backslash
  entry, NUL in name.
- Archive bombs: high ratio, huge total, many entries.
- Duplicate entry names, encrypted entry, CRC-corrupted entry.
- Symlink-to-outside archive; symlink-to-inside archive (allowed case).
- Oversized manifest; hostile id/name/version strings; ignored entry script.
- Fixtures are generated by a helper (checked-in bytes, no network).

### 20.4 FFI conformance (`tests/ffi_smoke.c`, CI)

- Compile a small C program against the staticlib.
- Exercise: `lda_api_version`, create/destroy (double-destroy of a nulled
  handle is safe), null-argument returns `LDA_ERR_INVALID_ARGUMENT`,
  list/install/uninstall round-trip on a temp root, `lda_free_string` /
  `lda_free_bytes` balance, storage dir returns a validated path, entry script
  bytes are non-empty and match the fixture.

### 20.5 Property tests (dev-dependency `proptest`)

- Ignore matcher vs a naive reference implementation over generated paths.
- Path normalization is total and never panics on arbitrary input.

### 20.6 Host-side (C++) tests

- No C++ test harness exists yet; the host keeps sandbox logic isolated in
  `addon_host.cpp` so it can be covered later. Minimal manual QA script in
  the addon system is acceptable for v1, with the sandbox contract documented
  in §13.

### 20.7 Hermeticity

- Every test uses `tempfile` dirs, pinned fixture bytes, and no network.
- Tests never depend on the real user config directory.

---

## 21. Extensibility Roadmap

| Item | When | Mechanism already in place |
|---|---|---|
| RemoteProvider (index + download + verify) | post-v1 | `AddonProvider` trait; `LzdContainer` signature flags; built-in key pinning |
| Addon signing (ed25519) | post-v1 | `.lzd` header reserves signature bit; verifier plugs into `archive.rs` |
| More host APIs (git context helpers, HTTP via host, custom UI panels) | post-v1 | versioned `lz` API; additive minor bumps |
| Per-addon permissions manifest | post-v1 | `schema_version` versioning; `[permissions]` additive field |
| Out-of-process addon isolation | post-v1 (research) | current sandbox is in-process; documented residual risk |
| Addon marketplace UI | post-v1 | provider seam + descriptors feed any UI |

---

## 22. Non-Goals (v1)

- No RemoteProvider implementation (trait only).
- No addon signing/verification (container reserves the format).
- No network access from addons (`lz.http` does not exist).
- No arbitrary binary/native addon plugins (Lua only).
- No addon-authored Qt widgets (commands + notifications only).
- No out-of-process sandboxing; addons run in-process under the Lua sandbox.
  Enable only trusted addons.
- No addon update channel/registry protocol.

---

## 23. Phase 2 Implementation Checklist

Implementation must follow this spec exactly. Ordered by dependency:

**Rust crate (`crates/addons/`)**
1. `Cargo.toml` (workspace member, staticlib, deps).
2. `error.rs` — taxonomy, JSON, sanitizer (+ unit tests).
3. `pathsec.rs` — normalization, containment, symlink policy (+ unit tests).
4. `ignore.rs` — matcher (+ unit + property tests).
5. `manifest.rs` — TOML schema + validation (+ unit tests).
6. `archive.rs` — zip/lzd parsing, limits, extraction (+ unit + security tests).
7. `package.rs` — descriptor, LoadedPackage, asset reader.
8. `provider/mod.rs`, `provider/local.rs`, `provider/remote.rs`.
9. `registry.rs` — snapshot index + operations.
10. `json.rs` — serde payloads.
11. `ffi.rs` — exports, catch_unwind, null checks, handle management.
12. `addons.h` — C ABI header.
13. `tests/ffi_smoke.c`, `tests/integration.rs`, `tests/security.rs`,
    `tests/fixtures/`.

**Build wiring**
14. Root `Cargo.toml` members; `xmake.lua` (before_build cargo, includes,
    link, lua package).

**C++/Qt**
15. `addon_manager_bridge.h/.cpp` — FFI wrapper, signals, QIcon assembly.
16. `addon_host.h/.cpp` — sandbox runtime, `lz.*` API, event dispatch.
17. `addons_dialog.h/.cpp` — manager UI; Addons menu in `mainwindow`.
18. Settings: enablement keys, storage.json handling.

**Verification**
19. `cargo test --workspace` green; `ffi_smoke.c` runs in CI; manual smoke:
    install a sample addon, run its command, uninstall.

**Close-out**
20. Delete this spec file; commit implementation; final report.
