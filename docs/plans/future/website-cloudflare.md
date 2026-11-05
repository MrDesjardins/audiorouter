# Website behind Cloudflare (DNS move, caching and security headers)

Status (2026-11-02): **future work, not authorized.** The owner chose to keep
GitHub Pages as the host and asked for a plan to move the DNS of
`audiorouter.org` to Cloudflare and proxy the site through it. Nothing below
runs until the owner approves it. The owner has to do the account and DNS
steps (Namecheap, Cloudflare, GitHub settings); an agent can do the
repository steps.

Origin: an outside report said "the audiorouter.org site is missing a
Content-Security-Policy header". The Pages-compatible part is done: every
page carries an enforced CSP `<meta>` tag (`site/generate-csp.mjs`, see
[site/README.md](../../../site/README.md#content-security-policy) and the
[active plan](../active/current.md#website-csp-2026-11-02-owner-request)).
GitHub Pages cannot send custom response headers, so the header itself and
`frame-ancestors` need a layer in front of Pages. This plan adds that layer.

Requirements touched: none of the product specifications (SEC-05 covers the
desktop shell, not the public site). This is website hosting and hardening.

## Goals

### Security

1. **Send a real `Content-Security-Policy` header** on every response,
   including GitHub's 404 page. Use the same policy as the meta tag plus
   `frame-ancestors 'none'`, which only works as a header and stops other
   sites from framing audiorouter.org (clickjacking). This clears the
   scanner finding.
2. **Add the other standard headers** that GitHub Pages leaves out:
   - `Strict-Transport-Security`, so browsers never try plain HTTP again.
     Start with a short `max-age` and raise it in steps (see step 7).
   - `X-Content-Type-Options: nosniff`
   - `Referrer-Policy: strict-origin-when-cross-origin`
   - `Permissions-Policy: camera=(), microphone=(), geolocation=(), payment=(), usb=()`
   - `X-Frame-Options: DENY` for old browsers that ignore `frame-ancestors`
3. **Leave the HTML exactly as built.** Cloudflare must not inject scripts or
   rewrite pages. Its script-injecting features (Rocket Loader, Web
   Analytics, Zaraz, email obfuscation, bot JavaScript challenges) would
   break the CSP and would add tracking. AudioRouter adds no telemetry
   without approval, and the site should follow the same rule.
4. **No cookies.** Responses must not carry `Set-Cookie`. The bot-management
   features that set `__cf_bm` stay off.
5. **Harden the domain itself:**
   - Turn on DNSSEC, so DNS answers for the domain are signed.
   - Verify `audiorouter.org` in GitHub (account Settings → Pages → Verified
     domains). Then no other GitHub account can claim the domain if Pages is
     ever unpublished.
   - Optionally add CAA records naming the certificate authorities that
     Cloudflare and GitHub use.
6. **Keep the free plan's DDoS protection and managed WAF rules on.**
   Nothing needs tuning for a static site.

### Caching and performance

1. **Serve from Cloudflare's edge**, closer to visitors than GitHub's CDN,
   with HTTP/3 and Brotli.
2. **Take load off GitHub Pages.** Pages has a soft limit of 100 GB of
   bandwidth per month. The guide videos alone are about 70 MB (154 MP4s
   under `site/assets/`), so a popular day adds up quickly. Requests that hit
   the edge cache never reach GitHub.
3. **Never show a stale page after a deploy.** File names under `assets/`
   have no content hashes (`assets/site.js`, `assets/site.css`), so long
   browser caching would pair new HTML with old CSS or JavaScript. The rule:
   long cache at the edge, a full purge on every deploy, and browsers keep
   GitHub's 10-minute `max-age`. Longer browser caching comes only after asset
   fingerprinting (optional step 9).
4. **The release list stays fresh.** `releases.html` and `download.html` load
   release data in the browser from `api.github.com`, so caching their HTML
   does not freeze the release list.

### Non-goals

- Moving the site off GitHub Pages, Cloudflare Pages, or Workers.
- Analytics of any kind, including Cloudflare Web Analytics.
- Proxying GitHub release downloads; they stay on `github.com`.
- HSTS preload. It is very hard to undo; reconsider only after a long stable
  period.

## Current state (measured 2026-11-02)

| Item | Value |
| --- | --- |
| Registrar and DNS | Namecheap BasicDNS (`dns1/dns2.registrar-servers.com`) |
| Apex `A` | `185.199.108.153`, `185.199.109.153`, `185.199.110.153`, `185.199.111.153` (GitHub Pages) |
| Apex `AAAA` | none |
| `www` | `CNAME mrdesjardins.github.io` |
| `MX` | `eforward4.registrar-servers.com` (15), `eforward5.registrar-servers.com` (20): Namecheap email forwarding |
| `TXT` | `v=spf1 include:spf.efwd.registrar-servers.com ~all`; `google-site-verification=vWsi5mT0XyO_p6uRSYA9ntxTEFcA3MLiF9s3Pzlpwgg` |
| `CAA`, DNSSEC, GitHub domain verification TXT | none |
| Response | `Server: GitHub.com`, `Cache-Control: max-age=600`, no CSP, HSTS or `nosniff` header; HTTP redirects to HTTPS (301) |
| Pages | custom domain from `site/CNAME`, "Enforce HTTPS" on, deployed by `.github/workflows/pages.yml` |

Check these again on the day of the move. The record list in Cloudflare must
match whatever is live then.

## Decisions to make before starting

1. **Email forwarding.** The `MX` records use Namecheap's forwarding service.
   Namecheap may not let you manage forwarding once the domain uses another
   DNS provider. Before the move, either confirm that copying the `MX` and SPF
   records keeps forwarding working (send a test mail after the move), or
   switch to Cloudflare Email Routing (free) and set up the same forwards
   there. Recommendation: Cloudflare Email Routing, so DNS and forwarding
   live in one place.
2. **Guide videos and Cloudflare's terms.** Cloudflare's service terms have
   limited serving video and other large media through the free CDN. Read the
   current terms on the day. If they still apply, add a Cache Rule that
   bypasses the cache for `*.mp4`. The videos still pass through the proxy and
   get the headers, but Cloudflare does not cache them. Record which choice
   you made.
3. **AI crawler controls.** Cloudflare can block AI crawlers and rewrite
   `robots.txt` by default on new zones. The site ships its own
   `site/robots.txt` from `generate-social.mjs`. Decide whether Cloudflare may
   change it. Recommendation: turn off the managed `robots.txt` so the
   repository stays the single source.

## Steps

Owner steps are marked **(owner)**. Each step says how to check it before
moving on.

### 1. Repository preparation (agent, before any DNS change)

- `site/generate-csp.mjs --header` prints the header value: the exported
  `policy` plus `; frame-ancestors 'none'`. Add a test in
  `site/generate-csp.test.mjs` that the header is the meta policy plus only
  that directive. The meta tag and the header are both enforced (the browser
  applies the stricter of the two), so they must not drift apart.
- Add `site/check-live-headers.mjs` (Node, no dependencies). Given a base URL,
  it checks:
  - `/`, a nested page (`/tools/gate.html`), an asset (`/assets/site.css`), an
    MP4 and a missing path (404). Each must carry the CSP header (equal to
    `--header`), HSTS, `nosniff`, `Referrer-Policy`, `Permissions-Policy`,
    `X-Frame-Options`, and no `Set-Cookie`.
  - `http://` returns a redirect to `https://`, and `www.` returns a redirect
    to the apex.
  - An MP4 range request returns `206`.
  - `cf-cache-status` is present (it proves the proxy is in the path).
  - The page body's `sha256` matches the deployed file, so Cloudflare did not
    rewrite the HTML. Compare against `site/` after running the same
    generators as `pages.yml` (catalogs, social, CSP). On 2026-11-02 they left
    every committed page unchanged, but run them anyway in case that changes.
  - The certificate behind Cloudflare: open TLS straight to a GitHub IP
    (`185.199.108.153:443`, SNI `audiorouter.org`) and print the expiry of
    GitHub's certificate (see risk 1).
- Add an optional purge step to `pages.yml` after `deploy-pages`. It runs only
  when the `CLOUDFLARE_API_TOKEN` secret and the `CLOUDFLARE_ZONE_ID` variable
  exist, so the workflow keeps working before and without Cloudflare:
  1. Poll the origin (a GitHub IP with `Host: audiorouter.org`) until
     `index.html` matches the uploaded artifact's hash, because GitHub's own
     CDN can serve the old copy for up to 10 minutes.
  2. Call `POST https://api.cloudflare.com/client/v4/zones/$ZONE/purge_cache`
     with `{"purge_everything":true}`.
  3. Run `check-live-headers.mjs https://audiorouter.org`.

  A failed purge or check is reported as a workflow warning, not a failed
  deploy. The site is already published by then, and a rollback to direct
  GitHub DNS must not turn every deploy red.
- Update `site/README.md` (hosting section) and this plan with what changed.
- Check: `node --test site/generate-csp.test.mjs`, `node tools/quality/verify.mjs
  --checks site,docs`, then run `check-live-headers.mjs` against the current
  site. It should fail on the missing headers but report the rest correctly.

### 2. Save the current DNS (owner)

- In Namecheap → Domain List → `audiorouter.org` → Advanced DNS, take a
  screenshot or copy every record. This is the rollback copy.
- Note the email forwards (Domain → Redirect Email).
- Check: the saved list matches the table above, or note the differences.

### 3. Create the Cloudflare zone (owner)

- Sign up or sign in at `dash.cloudflare.com`. Add a site: `audiorouter.org`,
  Free plan.
- Cloudflare scans the existing records. Compare them with step 2 and fix
  anything missing:

  | Type | Name | Content | Proxy |
  | --- | --- | --- | --- |
  | A | `@` | `185.199.108.153` | Proxied |
  | A | `@` | `185.199.109.153` | Proxied |
  | A | `@` | `185.199.110.153` | Proxied |
  | A | `@` | `185.199.111.153` | Proxied |
  | CNAME | `www` | `mrdesjardins.github.io` | Proxied |
  | MX | `@` | `eforward4.registrar-servers.com` (15) | DNS only |
  | MX | `@` | `eforward5.registrar-servers.com` (20) | DNS only |
  | TXT | `@` | `v=spf1 include:spf.efwd.registrar-servers.com ~all` | DNS only |
  | TXT | `@` | `google-site-verification=vWsi5mT0XyO_p6uRSYA9ntxTEFcA3MLiF9s3Pzlpwgg` | DNS only |

  If you chose Cloudflare Email Routing (decision 1), replace the MX and SPF
  rows with the records Email Routing creates, and set up the forwards there.
- Do **not** change nameservers yet. Configure steps 4 to 6 first, so the
  first visitor through Cloudflare already gets the final settings.

### 4. TLS settings (owner, Cloudflare dashboard)

- SSL/TLS → Overview: encryption mode **Full (strict)**. Never use Flexible:
  GitHub's "Enforce HTTPS" would redirect forever, and the hop to GitHub
  would be unencrypted.
- SSL/TLS → Edge Certificates: Always Use HTTPS **on**; Minimum TLS version
  **1.2**; TLS 1.3 **on**; Automatic HTTPS Rewrites **off** (it rewrites
  HTML, and nothing on the site needs it). Leave HSTS off for now (step 7).
- Keep GitHub Pages "Enforce HTTPS" on, and keep the custom domain.

### 5. Response headers (owner, Cloudflare dashboard)

Rules → Transform Rules → Modify Response Header → Create rule:

- Name: `AudioRouter security headers`. Match: all incoming requests.
- Operation **Set static** for each header:

  | Header | Value |
  | --- | --- |
  | `Content-Security-Policy` | output of `node site/generate-csp.mjs --header` (one line) |
  | `X-Content-Type-Options` | `nosniff` |
  | `Referrer-Policy` | `strict-origin-when-cross-origin` |
  | `Permissions-Policy` | `camera=(), microphone=(), geolocation=(), payment=(), usb=()` |
  | `X-Frame-Options` | `DENY` |

- `Permissions-Policy` was checked against the site on 2026-11-02: no page or
  script uses `getUserMedia`, `mediaDevices` or fullscreen calls, and the
  YouTube iframe's `allow=` list (accelerometer, autoplay, clipboard-write,
  encrypted-media, gyroscope, picture-in-picture, web-share) names none of the
  blocked features. If a page ever adds a live microphone demo, change that
  entry to `microphone=(self)`.
- When the policy in `generate-csp.mjs` changes, update this rule in the same
  change. `check-live-headers.mjs` fails until they match.

### 6. Caching, speed, and features to turn off (owner, Cloudflare dashboard)

Caching → Cache Rules (applied in this order):

1. `Guide videos` (only if decision 2 says so): URI path ends with `.mp4` →
   **Bypass cache**.
2. `Static site`: hostname equals `audiorouter.org` → Eligible for cache.
   Edge TTL: ignore the origin header, **1 month**. Browser TTL: **respect
   origin** (GitHub's 10 minutes). This also caches HTML, which Cloudflare
   does not do by default. The deploy purge (step 1) keeps it fresh.

Elsewhere:

- Caching → Tiered Cache: Smart Tiered Cache **on**. Always Online **off**.
- Speed: Brotli and HTTP/3 **on**. Rocket Loader **off**. Leave Cloudflare
  Fonts **off** (it rewrites HTML; the CSP already allows Google Fonts).
- Scrape Shield: Email Address Obfuscation **off**. Hotlink protection off.
- Security: Bot Fight Mode **off** (it injects JavaScript and sets cookies);
  leave the free managed WAF rules and DDoS protection at their defaults.
- Analytics → Web Analytics: **no** automatic setup. Zaraz: not enabled.
- AI crawl control / managed `robots.txt`: per decision 3.

### 7. Switch the nameservers (owner)

- Pick a quiet time. Both old and new DNS point at GitHub, so visitors see no
  outage during propagation: some reach GitHub directly, some go through
  Cloudflare.
- In Namecheap → Domain → Nameservers, choose **Custom DNS** and enter the
  two nameservers Cloudflare shows for the zone. Save.
- Wait for Cloudflare to email that the zone is **Active** (minutes to 48
  hours).
- Check from a terminal:

  ```powershell
  nslookup -type=NS audiorouter.org      # Cloudflare nameservers
  curl.exe -sI https://audiorouter.org/  # server: cloudflare, cf-cache-status, CSP
  node site/check-live-headers.mjs https://audiorouter.org
  ```

- Then, in Firefox (the owner's browser) with DevTools → Console open, check:
  the home page, a guide video playing, the YouTube embed on the home page,
  the Download page (latest release and sponsor button) and Releases. The
  console must show no CSP errors.
- Send a test email to a forwarded address.
- Turn on HSTS (SSL/TLS → Edge Certificates → HSTS) with `max-age` **1 day**,
  `includeSubDomains` off and preload off. After a week with no issues, raise
  it to 1 month, then to 6 months.

### 8. DNSSEC, domain verification, CAA, deploy purge (owner, then agent)

- Cloudflare → DNS → Settings → **Enable DNSSEC**. Copy the DS record into
  Namecheap → Domain → Advanced DNS → DNSSEC. Check:
  `nslookup -type=DS audiorouter.org` returns a record, and an online DNSSEC
  checker passes.
- GitHub → Settings (account) → Pages → **Add a verified domain**. Add the
  `_github-pages-challenge-MrDesjardins` TXT record it asks for in Cloudflare
  (DNS only) and click Verify.
- Optional CAA: add `0 issue "letsencrypt.org"` (GitHub's certificates) and
  `0 issue "pki.goog"`. Cloudflare adds the records its own certificates need
  when CAA records exist; check SSL/TLS → Edge Certificates still shows an
  active certificate afterwards.
- Create a Cloudflare API token: My Profile → API Tokens → Custom token, with
  permission **Zone → Cache Purge → Purge**, for the `audiorouter.org` zone
  only. In GitHub → repository Settings → Secrets and variables → Actions,
  add the secret `CLOUDFLARE_API_TOKEN` and the variable `CLOUDFLARE_ZONE_ID`
  (shown on the zone's Overview page).
- Check: push a small site change (or run the Pages workflow manually) and
  confirm the purge step ran, `check-live-headers.mjs` passed, and the change
  is live within a few minutes.

### 9. Optional later: fingerprinted assets

Have a generator rename `assets/site.css`, `assets/site.js` and friends to
hashed names (or add `?v=<hash>` to their URLs). Then give `assets/*`
`Cache-Control: public, max-age=31536000, immutable` with a Cache Rule.
Repeat visits stop re-checking every file. Do this only after the base move
has been stable for a while.

## Validation matrix

| Check | How | Pass |
| --- | --- | --- |
| CSP header on HTML, assets, 404 | `check-live-headers.mjs` | equals `generate-csp.mjs --header` |
| Other security headers, no cookies | `check-live-headers.mjs` | all present, no `Set-Cookie` |
| HTML unmodified by the proxy | body `sha256` vs `site/` after the `pages.yml` generators | equal |
| Redirects | same script | HTTP→HTTPS, `www`→apex |
| Media | same script, then Firefox | `206` on range request; video plays |
| Caching | `cf-cache-status` on a second request | `HIT` for HTML and assets (`BYPASS`/`DYNAMIC` for MP4 if bypassed) |
| Fresh after deploy | change a page, deploy | new content live within minutes |
| No CSP breakage | Firefox console on home, guide, download, releases | no violations |
| Email | test mail to a forward | delivered |
| DNSSEC | DS lookup and an online checker | signed, valid |
| Origin certificate | TLS straight to a GitHub IP | more than 30 days left |
| Scanner finding | re-run the reporter's scan, or securityheaders.com | CSP reported present |

## Risks

1. **GitHub's certificate renewal behind a proxy.** GitHub renews its Let's
   Encrypt certificate for the domain only while the domain resolves to
   GitHub; behind Cloudflare it sees Cloudflare's IPs. If renewal stops, the
   certificate between Cloudflare and GitHub expires, and **Full (strict)**
   shows visitors an error (526). `check-live-headers.mjs` prints that
   certificate's expiry. If it drops below 30 days and does not renew: set the
   apex and `www` records to **DNS only** for a day so GitHub can renew, then
   proxy again. Switching to **Full** (not strict) also accepts the expired
   certificate, but it no longer checks the origin. Use it only as a short
   emergency measure. Record what happens at the first renewal (about 60 to
   90 days after the move).
2. **Header and meta policy drift.** Both are enforced. A header with a
   narrower policy than the meta tag blocks resources the meta tag allows.
   Mitigation: one source (`generate-csp.mjs`), and the live check compares
   them.
3. **Stale content.** Covered by the purge step. If the token is missing or
   revoked, the edge serves old pages for up to the 1-month edge TTL: purge by
   hand (Caching → Configuration → Purge Everything) and fix the token.
4. **Email forwarding stops.** Covered by decision 1 and the test mail. To
   recover, restore the saved MX/SPF records, or set up Email Routing.
5. **Privacy.** Cloudflare now sees visitor IPs and requests, as GitHub
   already does. No analytics, no cookies, no injected scripts. If the site
   ever gets a privacy page, it should name both GitHub and Cloudflare.
6. **Lock-in of settings in a dashboard.** The settings live in Cloudflare,
   not in the repository. This plan is their record; update it whenever a
   setting changes.

## Rollback

- **Fast (minutes): stop proxying.** In Cloudflare DNS, set the apex and
  `www` records to **DNS only**. Traffic goes straight to GitHub again, with
  Cloudflare still answering DNS. The new headers disappear; the meta CSP
  stays.
- **Full: give DNS back to Namecheap.**
  1. **Remove the DS record at Namecheap first** and wait for its TTL to
     expire. Moving nameservers while a DS record still points at
     Cloudflare's keys makes the domain fail to resolve for validating
     resolvers.
  2. Namecheap → Nameservers → **Namecheap BasicDNS**.
  3. Re-create the records saved in step 2 under Advanced DNS (and the email
     forwards).
  4. Remove the purge secret and variable, or leave them: the workflow skips
     the purge when Cloudflare no longer serves the zone, as long as the
     step is written to treat an API failure as a warning.
- HSTS stays in visitors' browsers for its `max-age` either way. That is
  harmless because GitHub also serves HTTPS, and it is why HSTS starts at one
  day.

## Prerequisites and effort

- Owner: Namecheap login, a Cloudflare account (free), GitHub account and
  repository admin rights. About one hour of setup, then a wait for
  propagation and short checks over the following week.
- Agent: step 1 and step 8's check. No Windows-only, hardware or driver
  dependencies.

## Next action

When the owner approves: an agent does step 1 on its own branch and reports
the commands and results. The owner then works through steps 2 to 8 with
this page open, and the agent runs the validation matrix and records the
results here.
