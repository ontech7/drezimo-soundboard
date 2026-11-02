import { useEffect, useState } from "react";
import { useBoardActivity } from "./board";
import { TEXTS, TextsContext } from "./i18n";
import { BoardPage } from "./pages/BoardPage";
import { GeneralPage } from "./pages/GeneralPage";
import { OverlayPage } from "./pages/OverlayPage";
import { StatusPage } from "./pages/StatusPage";
import { useSettings } from "./settings";
import { type PageId, Sidebar } from "./Sidebar";
import { useStatus } from "./status";
import { windowReady } from "./window";

export default function App() {
  const status = useStatus();
  const settings = useSettings();
  // Kept here, so it goes on while another page is shown
  const activity = useBoardActivity(12);
  const [page, setPage] = useState<PageId>("status");
  const ready = status !== null && settings !== null;
  const language = settings?.shownLanguage ?? "english";

  // The first frame waits for the status and the settings, so the window never shows a state that changes at once
  useEffect(() => {
    if (ready) windowReady();
  }, [ready]);

  useEffect(() => {
    document.documentElement.lang = language === "italian" ? "it" : "en";
  }, [language]);

  if (!status || !settings) return null;
  const texts = TEXTS[language];

  return (
    <TextsContext.Provider value={texts}>
      <div className="app">
        <Sidebar page={page} onSelect={setPage} />
        <main>
          <h1>{texts.nav[page]}</h1>
          {page === "status" && <StatusPage status={status} activity={activity} />}
          {page === "board" && <BoardPage settings={settings} status={status} />}
          {page === "overlay" && <OverlayPage settings={settings} status={status} />}
          {page === "general" && <GeneralPage settings={settings} />}
        </main>
      </div>
    </TextsContext.Provider>
  );
}
