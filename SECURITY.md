# Security policy

## Reporting a vulnerability

Please don't open a public issue for security problems. Report them privately through GitHub: on the repository's **Security** tab, choose **Report a vulnerability**. Include what you found, how to reproduce it, and which version you used.

You'll get a reply as soon as possible, and fixes are shipped through the app's signed automatic updates.

## Supported versions

Only the latest release receives fixes. Kilat checks for updates by itself; you can also use **Settings → Check for updates**.

## How Kilat handles your data

- Tokens and API keys are stored in the operating system's secret store (Credential Manager on Windows, Keychain on macOS, Secret Service on Linux), never in plain files.
- Kilat has no servers and sends no telemetry. It only contacts Anthropic to verify Claude tokens you add, and GitHub (through `gh`) for account profiles.
- Updates are signed; the app only installs updates signed with the project's key.
