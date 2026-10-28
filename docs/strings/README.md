# Alert strings for reviewers

Firewatch's phone alerts can be written in **Hausa, Yoruba, Igbo and Nigerian Pidgin** as well as English. Each language needs one native speaker to review its words before it's offered. Until then it stays off: Firewatch refuses it unless `allow_draft_language = true` is set, and every alert then starts with *[Draft … translation, not reviewed yet]*.

| File | Language | Reviewer | Status |
|---|---|---|---|
| [alerts-ha.csv](alerts-ha.csv) | Hausa | _to confirm_ | draft |
| [alerts-yo.csv](alerts-yo.csv) | Yoruba | _to confirm_ | draft |
| [alerts-ig.csv](alerts-ig.csv) | Igbo | _to confirm_ | draft |
| [alerts-pcm.csv](alerts-pcm.csv) | Nigerian Pidgin | _to confirm_ | draft |

The drafts are first drafts by Dave. They aren't checked by a native speaker, and some will be wrong.

## What stays in English

Only Firewatch's own short phrases are translated. These stay in English, word for word, in every language:

- **The developers' fixes** ("Developers say: …"). They're quotes, and the promise is that we never reword them.
- **Advisory titles** and the **reason** sentences the engine writes. Translating them is planned for after the demo.
- **Names:** Firewatch, Lightning, LND, Core Lightning, rune, macaroon, npub, and node names like `old-cln`.

## For the reviewer

1. **Agree the key terms first** (B2), because the other phrases use them. Write your choice in the `notes` column of these rows:

   | Key | English | Meaning |
   |---|---|---|
   | `affected_label` | AFFECTED | The node has this security bug. |
   | `cant_tell_label` | CAN'T TELL | Firewatch can't confirm either way, so the operator must check by hand. It is not "safe". |
   | `no_longer_affected` | is no longer affected | Fixed, usually by upgrading. |
   | `check_by_hand` | Check by hand | The operator checks something themselves. |
   | `developers_say` | Developers say | Introduces the developers' own words, quoted in English. |

2. **Go through every row.** Read `english` and `where it appears`, correct the `draft`, and write the final text in **reviewed**. If the draft is already right, copy it across.
3. **Keep every placeholder** in `keep these placeholders` exactly as written, such as `{node}`, `{n}`, `{ids}`, `{since}`, `{detected}` and `{sent}`. Firewatch fills them in: `{node}` becomes a node name like `old-cln`, `{n}` a number, `{since}` a time like `14:02`. You can move them within the sentence.
4. **Keep it short.** The alerts are read on a phone. Plain, everyday words beat formal ones.
5. **Ideally, see it on a phone.** Ask Dave for a draft alert in your language, and check it reads naturally there, not just in the sheet.
6. Send the sheet back by **28 Oct, 12:00**.

## Putting reviewed text in (B7)

For each language whose sheet came back complete:

1. Copy the `reviewed` column into that language's table in [crates/firewatch-alerts/src/lang.rs](../../crates/firewatch-alerts/src/lang.rs), key by key.
2. Set `reviewed: true` and `reviewer` to the reviewer's name, as they want to be credited. A test fails if one is set without the other.
3. Run `cargo test -p firewatch-alerts`. It fails if any placeholder was lost or a phrase is empty.
4. Regenerate the sheets so they show the final text: `cargo run -p firewatch-alerts --example strings_sheet -- docs/strings`.
5. Update the table at the top of this page and the reviewer credits in the main README.

A language whose review isn't back on time stays a draft. It goes on the roadmap slide, not into the release.
