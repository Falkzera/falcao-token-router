# Security

## Reporting

Please don't open a public issue for anything with security or privacy impact.
Use the **Report a vulnerability** button under this repository's **Security**
tab: the report reaches only the maintainers.

Say what you saw, on which platform and app version, and how to reproduce it if
you can. Redact account e-mails, and never paste a credential or a token — nobody
here will ever ask for one.

## What counts

The app handles Claude Code credentials, so these matter most:

- a credential left where it shouldn't be — in another account's home, active in
  two places at once, or still on disk after its account or group was removed;
- a way for the app, its `router` CLI or its terminal integration to run
  something the user didn't ask for, or to write outside the files the platform
  READMEs list;
- any network call made by the app itself — it makes none, by design;
- any path by which a token could be decoded, logged or displayed.

## Supported versions

Fixes land on `main` and ship in the next release of each platform. Only the
latest release of each platform is supported.
