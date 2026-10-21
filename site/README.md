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

## Link previews, sitemap and robots.txt

`site/generate-social.mjs` adds a canonical URL and Open Graph/Twitter card
tags (title, description, `assets/social-preview.png`) to every page, between
`<!-- social -->` markers, and rewrites `sitemap.xml` and `robots.txt` from
the pages that exist. The Pages workflow runs it before deployment. Run it
after adding a page or regenerating the guides; `--check` reports files that
are out of date:

```powershell
node site/generate-social.mjs
```

Give every new page a `<title>` and a `<meta name="description">`; the
generator copies both into the preview tags.

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

## Visual tool guides

Open [the guide library](tools/index.html) locally, or use the static site preview. There are 35 pages: 34 audio node kinds plus visual Group. Every page includes a practical setup walkthrough, expected result, troubleshooting check, configuration references and worked examples. Relevant tools have locally generated synthetic audio sharing one Play/Stop and seek transport with their illustrations; the example repeats until Stop, a setting change is heard immediately (short crossfade), and time-axis pictures draw their traces from that audio so the playhead marks what is heard. Diagrams without a time axis have no playhead. The Edge suite (`ui/e2e/site-guides.pw.ts`) does not cover Firefox, where Web Audio is stricter; before publishing guide audio changes, also run `node tools/site/firefox-guide-audio.mjs file:///C:/code/audiorouter/site/tools/` with Firefox installed (every audible guide must play and keep playing through a control change). Device/application selection, arbitrary VST effects and Group remain visual-only.

Content lives in guide-content/processing.json, routing.json and tutorials.json. Source references identify the controls used to establish defaults and bounds. Update them when a control changes, then run:

    node site/generate-catalogs.mjs
    node site/check-guides.mjs

The catalog generator also generates individual HTML pages, the guide index and manifest. Keep generated pages committed so they can be reviewed as local files without a server. Animations use dependency-free native WAAPI. Controls are keyboard accessible. Reduced-motion visitors start with a static diagram; examples never autoplay. Playback pauses when the entire example leaves view, while its sliders and input references remain part of that example.

`guide-signal.js` produces deterministic PCM fixtures and bounded illustrative processing; `guide-audio.js` plays those samples and `guide-playback.js` shows their waveforms/readings. Mixer and Input Switch have distinct drums/bass and bell tracks; Duck has a separate control-only key. Input reference listening shares the same transport. Browser monitoring uses 50% gain with a peak-protection compressor. Recorder stores only its completed synthetic interval in page memory. No microphone, application capture, personal file, actual network stream or disk recording is used. Each guide explains its simulation limits; these examples are not engine or hardware qualification.

On Windows without Python, preview from the repository root:

```powershell
node .\ui\node_modules\vite\bin\vite.js .\site --host 127.0.0.1 --port 8000
```

Open `http://127.0.0.1:8000/tools/`. Vite's site directory is a positional argument.

Tool cards have a compact question mark inside their visual boundary; Properties also links to the corresponding guide. Every desktop build, repository builds included, opens https://audiorouter.org/tools/<tool>.html; review unpublished guide edits by opening site/tools/<tool>.html directly. These edits do not change the published site until the existing Pages workflow runs through an authorized push/merge or manual deployment.
