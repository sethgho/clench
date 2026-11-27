# Fixtures

Snapshots of real web pages (and one PDF) used to regression-test extraction.
They are third-party content, saved verbatim from the URL in each `.url` file
and kept here for local regression testing only; they are not redistributed
as part of meatread and remain the property of their publishers. They are not
covered by this repository's MIT License.

| Fixture | Page | Failure class it guards against |
|---|---|---|
| `01-iankduncan` | Personal blog essay | Author box, "keep reading" and related posts, footer, image credit outside a `<figure>`, standfirst outside the scored container. The acceptance case. |
| `02-news-arstechnica` | News feature | Headline kicker, dek, closing author note and syndication line, comments and "most read" modules. |
| `03-substack-acx` | Newsletter post | Subscribe, share and restack widgets, comment counts. |
| `04-ghost-404media` | Paywalled post | Truncated body behind a subscription prompt: must be flagged `partial`, not "fixed". |
| `05-static-matklad` | Static-site blog | Code blocks without a language, a standalone dateline in the header, lists and a blockquote. |
| `06-wikipedia` | Encyclopedia article | Footnotes with back-links, ordered and unordered lists, blockquotes, maintenance markers, infobox and navigation. |
| `07-rustbook` | mdBook chapter | Many `<pre>` blocks with `language-*` classes that must survive byte for byte, keyboard-help overlays. |
| `08-mdn` | API reference | Data tables, `brush:` style code languages, "see also" lists. |
| `09-github-readme` | Rendered README | Application chrome around rendered Markdown; code and lists. |
| `10-arxiv-abs` | Paper landing page | A metadata-heavy page whose article is the abstract: returned from `citation_abstract`, flagged `partial`. |
| `11-paulgraham` | Legacy essay | `<br><br>` paragraphs, layout tables, bracketed endnotes. |
| `12-recipe-allrecipes` | Recipe | Ingredient and step lists, photo captions, ad and newsletter furniture. |
| `13-hn-thread` | Discussion thread | Table-based layout (not data tables), many short comments. |
| `14-nonenglish-spiegel` | German news article | Non-English text, `isAccessibleForFree:false` on a page that serves the full text (`paywall_hint` without `partial`), a teaser outside the body, a recommendation module inside it. |
| `15-spa-bsky` | Client-rendered post | The text is not in the HTML at all: only the meta description exists, so the result is flagged `partial`. |
| `16-news-bbc` | News article | Photo credits, related links, share and save controls, relative timestamps. |
| `attention.pdf` | Sample paper | PDF text layer: hard-wrapped lines, numbered section headings, page numbers, tables and math. |

`truth.json` holds, per page, the first and last sentence of the real article
(`start`, `end`), passages that must be present (`must`, which may live in
code blocks) and chrome strings that must not be read (`junk`). `start`,
`end` and `must` match case-insensitively with typographic quotes
straightened; `junk` matches case-sensitively.

Corrections made to the original ground truth:

- `01-iankduncan` `start` is the lede "LLMs mostly amplify whoever is sitting
  at the keyboard." The page's standfirst belongs to the essay.
- `02-news-arstechnica` `end` was a sentence in the middle of the closing
  section; it is now the article's real last sentence. The junk entry
  "Shut up and calculate" was removed because the phrase is also ordinary
  article text (the headline kicker is checked separately), and the closing
  author note and syndication line were added as junk.

Where output deliberately differs from the truth file, the expectation is
recorded in `crates/meatread-extract/tests/fixtures.rs` rather than left
untested.
