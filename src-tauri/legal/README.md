# isoboard legal documents

**isoboard-terms-v2 · 2026-11-13**

These files, together with the English source license in the repository root
[LICENSE.md](../../LICENSE.md), are the editable source of truth for the release's
first-party terms and privacy information. The website renders them from an app
submodule pinned to a release, and customer PDFs are generated from them; make
wording changes here.
English and Spanish have the same package version. Privacy notices are
informational; they are not blanket consent or additional license restrictions.

| Document            | English                           | Español                              |
| ------------------- | --------------------------------- | ------------------------------------ |
| Source license      | [English](../../LICENSE.md)       | [Español](es/SOURCE-LICENSE.md)      |
| Commercial terms    | [English](COMMERCIAL-TERMS.md)    | [Español](es/COMMERCIAL-TERMS.md)    |
| Purchase refunds    | [English](PURCHASE-REFUNDS.md)    | [Español](es/PURCHASE-REFUNDS.md)    |
| Support             | [English](SUPPORT.md)             | [Español](es/SUPPORT.md)             |
| Branding forks      | [English](BRANDING-FORKS.md)      | [Español](es/BRANDING-FORKS.md)      |
| Privacy             | [English](PRIVACY-NOTICE.md)      | [Español](es/PRIVACY-NOTICE.md)      |
| Legal notice        | [English](LEGAL-NOTICE.md)        | [Español](es/LEGAL-NOTICE.md)        |
| Withdrawal form     | [English](WITHDRAWAL-FORM.md)     | [Español](es/WITHDRAWAL-FORM.md)     |
| Website storage     | [English](WEBSITE-STORAGE.md)     | [Español](es/WEBSITE-STORAGE.md)     |
| System requirements | [English](SYSTEM-REQUIREMENTS.md) | [Español](es/SYSTEM-REQUIREMENTS.md) |

The English source license exists only as the root LICENSE.md. When the app is
built, `src-tauri/build.rs` stages this folder in `src-tauri/gen/legal/` for
bundling: it adds LICENSE.md as SOURCE-LICENSE.md and THIRD_PARTY_NOTICES.txt,
rewrites links to LICENSE.md for the bundle layout, and fails the build if a
bundled document links to a missing file. Package version/date and language
links are presentation metadata; the document grants are unchanged.

The app's License actions open SOURCE-LICENSE.md, COMMERCIAL-TERMS.md and
PRIVACY-NOTICE.md in the system's default viewer. Supporting documents and the
Spanish copies are included in the same resource directory. If a plain-text
viewer cannot follow Markdown links, open the named adjacent files directly.

Third-party components and models retain their separate licenses. The notices,
MPL license/source-access instructions, source archives and toolchain texts in
this directory are not relicensed under the isoboard terms. Previously granted
rights in earlier copies are not retroactively withdrawn by this package.
