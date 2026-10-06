# AudioRouter website

Static GitHub Pages site for [audiorouter.org](https://audiorouter.org).

## Publish

The `pages.yml` workflow deploys the contents of this folder to GitHub Pages
after changes reach `main`, and supports manual runs. In the repository, open
**Settings → Pages** and choose **GitHub Actions** as the build and deployment
source. Set the custom domain there to `audiorouter.org`. With a custom Actions
workflow, the GitHub Pages setting is authoritative; the `site/CNAME` file is
not required to configure the domain.

At your domain registrar, create these DNS records (remove conflicting apex
records that point elsewhere):

- Four `A` records for host `@`: `185.199.108.153`, `185.199.109.153`,
  `185.199.110.153`, and `185.199.111.153`.
- Optional `CNAME` record for `www` pointing to `MrDesjardins.github.io`, if you
  want `www.audiorouter.org` to work too.

GitHub recommends verifying ownership of the domain in account settings before
adding it to the repository. Once DNS resolves and Pages issues the certificate,
return to **Settings → Pages** and enable **Enforce HTTPS**. DNS changes can take
up to 24 hours to propagate.

## Latest installer link

The Download buttons query the public GitHub Releases API on page load and
select the newest published release asset named like
`AudioRouter_*_x64-setup.exe`. This includes prereleases. If GitHub's API is
unavailable or no matching asset exists, links fall back to the releases page.
Keep this asset naming convention for future Windows setup packages.

The API resolution means publishing a release does not require editing the
website to bump a versioned download URL.

`releases.html` loads the latest 20 published release notes from the public
GitHub Releases API when opened. The page falls back to the GitHub Releases
listing if that feed is unavailable. The dedicated Stream Deck page describes
the local controls and first-time connection.

## Feature catalogs

`site/generate-catalogs.mjs` reads the current backend method contract and
rewrites the public static API operation and available node-tool catalogs,
including app-matching tool glyph identifiers.
The website includes HTTP aliases alongside public backend methods; desktop
consent controls are excluded. Run it from the repository root after changing
API methods or node kinds:

```powershell
node site/generate-catalogs.mjs
```

The Pages workflow runs this generator before deployment. The API page links
to the live Swagger UI in the user's local AudioRouter instance for full
request and response schemas.

The homepage embeds the requested AudioRouter video and includes a real Duck inspector screenshot from project UI evidence. `stream-deck.html` explains the local Stream Deck actions and first-time API connection. `siege.html` documents Stats.cc round ducking, phase choices, and the labeled-take EQ/compressor experiments; its example omits device and session identifiers.

## Homepage setup video

The "This is what it looks like running" section on `index.html` shows a
poster image first, then fades in a muted, looping video once it is playing.
Add two files:

- `site/assets/setup.jpg`: the poster. Use the video's first frame so the swap
  is invisible.
- `site/assets/setup.mp4`: H.264, no audio track, `+faststart`, ideally under
  about 8 MB.

Both are drawn in the same 16:9 frame (`object-fit: cover`). For a different
shape, set `style="--media-ratio:4/3"` (for example) on the
`[data-setup-media]` figure and update the `<img>` `width`/`height`.

```powershell
ffmpeg -i my-recording.mp4 -an -vf "scale=1600:-2,fps=30" -c:v libx264 -pix_fmt yuv420p -crf 26 -preset slow -movflags +faststart site/assets/setup.mp4
ffmpeg -i site/assets/setup.mp4 -frames:v 1 -q:v 3 site/assets/setup.jpg
```

`site.js` starts the video download only after the page has loaded and the
frame is near the viewport. It pauses the video while it is off-screen. It
skips the video entirely for reduced-motion or data-saver visitors, so they see
only the poster. On `localhost`, a missing poster shows a "Setup preview"
placeholder. On the live site, the frame and its heading are hidden until the
files exist.

## Content boundary

Feature descriptions reflect the repository specs and current public release
notes. The site describes compatibility with existing virtual endpoints and
does not promise every VST plugin is compatible. Third-party plugins are not
bundled. Download messaging notes that current releases are unsigned.
