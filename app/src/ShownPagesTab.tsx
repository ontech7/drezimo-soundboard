import { Row, Section, Toggle } from "./controls";
import { useCustomPages, useLibrary } from "./customPages";
import { useTexts } from "./i18n";
import { type SettingsView, saveSettings, type ShownPages } from "./settings";

/** Which of Voicemod's pages the board shows after the user's own, on the Drezimo page */
export function ShownPagesTab({ settings, onOpenPages }: { settings: SettingsView; onOpenPages: () => void }) {
  const texts = useTexts();
  const tab = texts.shownPagesTab;
  const library = useLibrary();
  const [pages] = useCustomPages();
  const shown = settings.shownPages;
  const save = (changed: Partial<ShownPages>) => saveSettings({ ...settings, shownPages: { ...shown, ...changed } });
  const soundboards = library?.soundboards ?? [];
  const own = pages?.length ?? 0;

  return (
    <>
      <Section title={tab.title} text={tab.text}>
        <Row label={tab.yourPages} text={own ? tab.yourPagesCount(own) : tab.noPagesYet}>
          <button type="button" className="link" onClick={onOpenPages}>
            {tab.editPages}
          </button>
        </Row>
        <Row label={texts.favouritesPage} text={library?.favourites ? tab.favouritesText : tab.noFavourites}>
          <Toggle label={texts.favouritesPage} checked={shown.favourites} onChange={(favourites) => save({ favourites })} />
        </Row>
        <Row label={texts.voicesPage} text={tab.voicesText}>
          <Toggle label={texts.voicesPage} checked={shown.voices} onChange={(voices) => save({ voices })} />
        </Row>
        <Row label={tab.onlyMine} text={tab.onlyMineText}>
          <Toggle label={tab.onlyMine} checked={shown.onlyMine} onChange={(onlyMine) => save({ onlyMine })} />
        </Row>
      </Section>

      <Section title={tab.soundboards} text={soundboards.length ? tab.soundboardsText : texts.voicemodNeeded}>
        {soundboards.map((soundboard) => {
          // Left out by "only mine", whatever its own switch says
          const blocked = shown.onlyMine && !soundboard.custom;
          const on = !blocked && !shown.hidden.includes(soundboard.id);
          const kind = soundboard.custom ? tab.madeByYou : tab.voicemods;
          return (
            <Row
              key={soundboard.id}
              label={soundboard.name}
              text={`${kind} · ${texts.soundCount(soundboard.sounds.length)}`}
              disabled={blocked}
            >
              <Toggle
                label={soundboard.name}
                checked={on}
                disabled={blocked}
                onChange={(checked) =>
                  save({
                    hidden: checked ? shown.hidden.filter((id) => id !== soundboard.id) : [...shown.hidden, soundboard.id],
                  })
                }
              />
            </Row>
          );
        })}
      </Section>
    </>
  );
}
