# Sponsorship (GitHub Sponsors)

Purpose: fund the Windows signing of AudioRouter's free virtual cable.
Microsoft only signs a driver submitted with an EV code-signing certificate
tied to the Hardware Dev Center account, and that certificate is the
project's one recurring cost ([signing runbook](virtual-cable-signing.md),
[driver signing plan](../plans/future/M03-driver-signing.md)).

Where people find it:

- the **Sponsor** button on the repository (`.github/FUNDING.yml`);
- the download page, [audiorouter.org/download.html](https://audiorouter.org/download.html)
  (`site/download.html`): every Download button on the site leads there, and
  the footer's **Sponsor** link opens its `#support` section;
- the README's "Support AudioRouter" section;
- later, the virtual cable download or setup page, once it exists (add the
  same short explanation and link there).

## Set up the profile (owner, in the GitHub web dashboard)

The listing `sponsors-MrDesjardins` exists and is public (checked
2026-10-13). Tiers, the goal and payout details are set at
**github.com/sponsors/MrDesjardins/dashboard**; they cannot be set from the
repository.

1. **Profile → Introduction**, paste:

   > I build AudioRouter, a free and open source audio router for Windows.
   > AudioRouter is adding its own virtual cable, so you will not need a
   > separate cable driver to send your voice or game audio to Discord, OBS
   > or other apps. Windows only installs audio drivers that Microsoft has
   > signed, and Microsoft requires an EV code-signing certificate for every
   > driver submission. Sponsorships fund that certificate and its renewals.
   > Everyone gets the same free software: no paid features, no priority
   > support. Money beyond the goal goes to future renewals and project
   > costs, and I post an update when the certificate is bought.

2. **Tiers**, enable one-time and monthly, and create:

   | Amount | Type | Name | Description |
   | --- | --- | --- | --- |
   | $25 | One-time | Cable supporter | Covers a real share of the signing certificate. Optional name in the project's acknowledgments. |
   | $5 | One-time | Say thanks | A small contribution toward the certificate. |
   | $100 | One-time | Signing sponsor | Funds a large part of the first year. Optional acknowledgment with a link. |
   | $5 | Monthly | Keep AudioRouter free | Helps with certificate renewals and ongoing project costs. |

   Do not add rewards such as priority support, custom features or release
   dates; recognition only.

3. **Goal** (set 2026-10-13): GitHub offers only monthly goals (amount or
   number of sponsors). The goal is **$45 a month** (about $540 a year),
   matching the plan's estimate of $280–$450 for the certificate plus $0–$120
   for key storage. One-time sponsorships do not count toward a monthly goal
   but fund the first purchase directly; say so on the download page.

Tiers created 2026-10-13 and their direct links (used by the download page;
a signed-out visitor goes through GitHub sign-in back to the tier):

| Tier | Link |
| --- | --- |
| $25 one time, Cable supporter | `https://github.com/sponsors/MrDesjardins/sponsorships?tier_id=667660` |
| $5 a month, Keep AudioRouter free | `https://github.com/sponsors/MrDesjardins/sponsorships?tier_id=664358` |
| $5 one time, Say thanks | `https://github.com/sponsors/MrDesjardins/sponsorships?tier_id=664359` |
| $100 one time, Signing sponsor | `https://github.com/sponsors/MrDesjardins/sponsorships?tier_id=667661` |

The numeric `tier_id` is the last four bytes of each tier's GraphQL node ID
(`gh api graphql` → `sponsorsListing.tiers`). If a tier is recreated, its
link changes.

Embeds: the download page uses GitHub's small Sponsor button
(`/sponsors/MrDesjardins/button`). The larger card
(`/sponsors/MrDesjardins/card`) was tried and left out: it is a mostly empty
white 600 × 225 box on the dark site. Both embeds load only over http(s),
not from a page opened as a local file.

## Transparency

- Keep the goal and the amount received visible on the Sponsors page.
- When the certificate is bought, post a short update (Sponsors update and a
  release note or README line): what was bought, the cost, and what is next.
- If contributions exceed the goal, say so and state that the balance funds
  renewals and project costs.
- Acknowledgments list only sponsors who opted in.
