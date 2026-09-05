# Third-party notices

- **RuoYi-Go BY** — https://github.com/touensan/ruoyi-go-by, commit `fbaf4bf964912ed972d51217ecd6f50acef692a6`. Main frontend/API/schema reference. Upstream README declares MIT. This repository's server is newly implemented in Rust.
- **RuoYi-PHP BY** — https://github.com/touensan/ruoyi-php-by, commit `caf414ab0080d5c5982d583f9736249fefbdbb3c`, MIT, copyright 2026 touensan. Reuses its sanitized schema/seed and RuoYi frontend adaptations; no PHP backend runtime or Laravel source is included.
- **RuoYi frontend** — MIT, copyright 2018 RuoYi. Original notice is retained in `frontend/LICENSE`. References: https://gitee.com/y_project/RuoYi-Vue and the Vue 3 TypeScript frontend distributed by RuoYi-Go BY. The Gitee endpoint returned HTTP 429/timeouts during this review; the frontend baseline is pinned to the available Go repository.
- **Qiluo Admin** — https://github.com/chelunfu/qiluo_admin, commit `cf389ed47eba865f0917e9765c798c2bce41bfdf`, MIT, copyright 2024 Open. Architecture reference (Rust/Axum, backend authorization, revocable tokens), not a source-code copy. Its SaaS, WeChat, AI and payment modules are not included.

Cargo and npm dependencies retain their respective licenses. The repository's MIT license does not replace dependency licenses. Dependency versions are recorded in Cargo.lock and frontend/package-lock.json. No production records, real credentials or private server configuration are included.

Redistributed license texts and package attribution metadata are bundled in `docs/DEPENDENCY_LICENSES.txt`, including target-specific Cargo packages that may not be linked on Linux. For packages whose published archives contain no standalone license file, their published metadata and available README license section are recorded.
