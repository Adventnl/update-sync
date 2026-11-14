# Security and trust

Relay installs code from repositories you choose. A matching checksum proves that a download matches GitHub's metadata. It does not prove that the author or application is safe. Checksum acceptance, platform signature status, and build trust are separate decisions.

GitHub CLI credentials are read from `gh auth token --hostname github.com` when enabled. Optional tokens are stored in Windows Credential Manager or the macOS Keychain through the keyring library. Tokens are never persisted in SQLite, sent to the webview, embedded in repository URLs, or inherited by build commands. Repository code still runs with your account permissions and can read other files available to that account. Local builds therefore require explicit trust.

Repository URLs must be plain HTTPS github.com owner/repository URLs. Download requests start at GitHub API or GitHub URLs. Redirects must remain HTTPS and have a hop limit. The HTTP client strips sensitive authentication on cross-origin redirects. No remote scripts or web content are loaded in the application webview.

Windows checks Authenticode when present and rejects invalid signatures. Downloaded launch files go through Windows Attachment Services before activation, which preserves origin evidence and applies configured attachment security. Native EXEs open through the Windows shell. Unsigned portable binaries can be installed after package trust review and may prompt through Windows security. macOS `.app` packages must pass strict code-signature verification and Gatekeeper assessment. Relay does not remove quarantine attributes or disable Gatekeeper. Java archives use package digest verification and the installed Java runtime. See Microsoft's [attachment save contract](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-iattachmentexecute-save) and Apple's [distribution guidance](https://developer.apple.com/macos/distribution/).

Relay does not claim to sandbox installed applications or build scripts. It does not run privileged installers, add drivers, install services, clear global package caches, or guess uninstall commands. Such installers need a dedicated adapter with a native uninstall identity and its own recovery policy before they can be managed safely.

Ownership checks protect against accidental deletion and path traversal. They are not a security boundary against an attacker already running as the same user, who can alter Relay's database or files. Relay runs without administrator rights and never elevates its cleanup process.

External user-data removal is opt-in. A folder must be explicitly selected, lie in an allowed per-user data area, avoid shared platform folders and overlap with other apps, and carry a matching ownership marker. Missing or changed markers stop removal. Unregistered data is kept.

## Release requirements

The local Windows build is unsigned. Sign Windows distributions with your certificate before distributing them beyond your own computers. Configure Apple Developer ID signing and notarization for macOS packaging. No signing credentials are included in the repository. macOS integration and packaged app behavior need testing on a Mac before treating that platform as production-verified.
