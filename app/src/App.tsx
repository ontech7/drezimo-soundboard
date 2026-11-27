import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { useBoardActivity } from "./board";
import { useBoardView } from "./boardView";
import { TEXTS, TextsContext } from "./i18n";
import { BoardPage, type BoardTab } from "./pages/BoardPage";
import { DiagnosticsPage } from "./pages/DiagnosticsPage";
import { SettingsPage } from "./pages/SettingsPage";
import { MacrosPage } from "./pages/MacrosPage";
import { OverlayPage } from "./pages/OverlayPage";
import { PagesPage } from "./pages/PagesPage";
import { SoundboardPage, SoundboardTitle } from "./pages/SoundboardPage";
import { DisplayButton } from "./DisplayPreview";
import { VirtualKnob } from "./VirtualKnob";
import { saveSettings, useSettings } from "./settings";
import { connections, type PageId, Sidebar } from "./Sidebar";
import { useStatus } from "./status";
import { Tour } from "./tour/Tour";
import { windowReady } from "./window";

const HINT_TIME = 8000;

export default function App() {
  const status = useStatus();
  const settings = useSettings();
  // Kept here, so they go on while another page is shown
  const activity = useBoardActivity(12);
  const view = useBoardView();
  const [page, setPage] = useState<PageId>("soundboard");
  const [boardTab, setBoardTab] = useState<BoardTab>("pad");
  // A page of the user's own to open in the Pages editor, from the Soundboard page
  const [editing, setEditing] = useState<string | null>(null);
  // Until opened or closed here, the tour shows while not seen yet: at the first start, in the first frame
  const [tour, setTour] = useState<boolean | null>(null);
  // Where to find the tour again, once it is skipped
  const [hint, setHint] = useState(false);
  const ready = status !== null && settings !== null;
  const language = settings?.shownLanguage ?? "english";

  // The first frame waits for the status and the settings, so the window never shows a state that changes at once
  useEffect(() => {
    if (ready) windowReady();
  }, [ready]);

  useEffect(() => {
    if (!hint) return;
    const timer = setTimeout(() => setHint(false), HINT_TIME);
    return () => clearTimeout(timer);
  }, [hint]);

  const tourShown = settings !== null && (tour ?? !settings.tourSeen);

  // Closing the window with the tour on counts as having seen it: the window would open again at each start otherwise
  useEffect(() => {
    if (!settings || !tourShown || settings.tourSeen) return;
    const unlisten = listen("window-closed", () => {
      setTour(true);
      saveSettings({ ...settings, tourSeen: true });
    });
    return () => {
      unlisten.then((stop) => stop()).catch(() => {});
    };
  }, [settings, tourShown]);

  useEffect(() => {
    document.documentElement.lang = language === "italian" ? "it" : "en";
  }, [language]);

  if (!status || !settings) return null;
  const texts = TEXTS[language];
  const pages: PageId[] = ["soundboard", "pages", "macros", "board", "overlay"];
  if (settings.diagnostics) pages.push("diagnostics");
  // Diagnostics turned off while shown
  const shown = page === "settings" || pages.includes(page) ? page : "soundboard";
  const boardOwnPage = view && view.page < view.own ? view.ids[view.page] : null;
  const open = (next: PageId) => {
    setEditing(null);
    setPage(next);
  };

  return (
    <TextsContext.Provider value={texts}>
      <div className="app">
        <Sidebar pages={pages} page={shown} connections={connections(texts, status)} onSelect={open} />
        <main>
          <header className="page-header">
            {shown === "soundboard" ? <SoundboardTitle status={status} playable={!!view} /> : <h1>{texts.nav[shown]}</h1>}
            {shown === "soundboard" && (
              <div className="page-tools">
                <DisplayButton />
                <VirtualKnob />
              </div>
            )}
          </header>
          {shown === "soundboard" && (
            <SoundboardPage
              status={status}
              settings={settings}
              view={view}
              onMakePage={() => open("pages")}
              onEditPage={(id) => {
                setEditing(id);
                setPage("pages");
              }}
            />
          )}
          {shown === "pages" && (
            <PagesPage
              view={view ?? null}
              open={editing ?? boardOwnPage}
              shownPages={settings.shownPages}
              pressAgainByMode={settings.pressAgainByMode}
              singleStop={status.singleSoundStop}
              onOpenMacros={() => open("macros")}
            />
          )}
          {shown === "macros" && <MacrosPage status={status} />}
          {shown === "board" && (
            <BoardPage
              settings={settings}
              status={status}
              tab={boardTab}
              onTab={setBoardTab}
              onOpenPages={() => open("pages")}
            />
          )}
          {shown === "overlay" && <OverlayPage settings={settings} status={status} />}
          {shown === "diagnostics" && <DiagnosticsPage status={status} activity={activity} />}
          {shown === "settings" && <SettingsPage settings={settings} status={status} onTour={() => setTour(true)} />}
        </main>
      </div>
      {tourShown && (
        <Tour
          onClose={(finished) => {
            setTour(false);
            setHint(!finished && !settings.tourSeen);
            if (!settings.tourSeen) saveSettings({ ...settings, tourSeen: true });
          }}
        />
      )}
      {hint && (
        <div className="toast" role="status">
          {texts.tour.later}
          <button
            type="button"
            onClick={() => {
              setHint(false);
              setTour(true);
            }}
          >
            {texts.tour.watch}
          </button>
        </div>
      )}
    </TextsContext.Provider>
  );
}
