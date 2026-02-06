# Scrubby Launch Plan (Tomorrow)

## Quick Answers
- **Do I need a GitHub repo?** Not strictly, but it’s the fastest way to host binaries and the landing page. Recommended.
- **Will Stripe protect licenses?** No. Payment is Stripe; license enforcement is on us.
- **Can it be hacked?** Any offline license can be shared or patched. We can deter, not prevent.

## Recommended Hosting (Fastest Today)
1. **GitHub Releases**
   - Upload `scrubby` binary and `checksums.txt`.
2. **GitHub Pages**
   - Host `docs/index.html` as the landing page.
3. **Stripe**
   - Use Stripe Checkout + webhook for automated license delivery.
4. **Webhook hosting (fastest today)**
   - Render Web Service for the webhook.

## What We Ship
- v0.1 Free: `scrubby --clipboard`
- v0.1 Pro (build with flags):
  - `--stable`, `--json`, `--config`, `--stdin`, `--file`
- Licensing: local file at `~/.config/scrubby/license.key`.

## License Model (Current)
**Signed license files (offline):**
- Build with embedded public key via `SCRUBBY_PUBLIC_KEY_B64`.
- License file is signed with your private key.
- `SCRUBBY_LICENSE=DEV` works only in debug builds.

## Mitigations (If You Want Stronger Later)
Already using signed licenses. Optional next steps:
- Display purchaser email on Pro usage (watermark).
- Soft device limit (e.g., 2 devices) with warnings.

## Release Checklist
1. Generate a keypair (keep private key secret):
   ```bash
   cargo run --bin license_keygen
   ```
   - Save `PRIVATE_KEY_B64` securely (do not commit).
   - Use `PUBLIC_KEY_B64` to build the binary.
2. Build Pro:
   ```bash
   SCRUBBY_PUBLIC_KEY_B64=<PUBLIC_KEY_B64> cargo build --release --features pro-stable-placeholders,pro-json-report,pro-config,pro-file-stdin
   ```
3. Ask customer to run:
   ```bash
   ./target/release/scrubby --device-id
   # or after install:
   # ./scripts/install-local.sh && scrubby --device-id
   ```
   and send you the output.
4. Create a license file for a customer:
   ```bash
   SCRUBBY_PRIVATE_KEY_B64=<PRIVATE_KEY_B64> cargo run --bin license_sign -- --email user@example.com --plan pro --device-id <DEVICE_ID> --out license.key
   ```
5. Deliver `license.key` to the customer.
6. Generate checksum:
   ```bash
   sha256sum target/release/scrubby > checksums.txt
   ```
7. Create GitHub Release:
   - Attach `scrubby` and `checksums.txt`.
8. Enable GitHub Pages:
   - Use `/docs` folder (landing page).
9. Create Stripe Checkout product and test payment.

## Automated License Delivery (Stripe)
- Use the separate **license-minter** service repo.
- Create a Stripe Checkout product.
- Add `device_id` to Checkout `metadata`.
- Listen for `checkout.session.completed` webhook.
- Webhook generates device-bound `license.key`.

### License Minter Service
- Repo: `https://github.com/kevintatou/license-minter`
- Deploy with its `render.yaml`.

### Local E2E Test (No external services)
Run tests inside the `license-minter` repo.

## End-to-End Test (Stripe Test Mode)
1. Generate a device id:
   ```bash
   ./target/release/scrubby --device-id
   ```
2. Create a Stripe test Checkout Session or Payment Link and set metadata:
   - `device_id=<value>`
3. Complete a test payment using:
   - `4242 4242 4242 4242`
4. Check Render logs for a 200 OK from `/webhook`.
5. Confirm the license file was written in `SCRUBBY_LICENSE_OUT_DIR` on the server.


## Communication Notes
- State clearly: Pro licenses are device-bound.
- Offer re-issue on device changes.

## Customer Instructions (Pro)
1. Download the Pro binary.
2. Place `license.key` at:
   - `~/.config/scrubby/license.key`
3. Run Pro features:
   ```bash
   scrubby --stable
   scrubby --json
   scrubby --config ./scrubby.conf
   ```

## FAQ Snippets
**Q: Does Scrubby send any data to the cloud?**
A: No. All detection and redaction is local.

**Q: Can I run watch mode all the time?**
A: It’s experimental and opt-in only. Use `scrubby --watch` if needed.
