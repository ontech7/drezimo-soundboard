import { useEffect, useState } from "react";
import { useBoardActivity } from "./board";
import { TEXTS, TextsContext } from "./i18n";
import { BoardPage } from "./pages/BoardPage";
import { DiagnosticsPage } from "./pages/DiagnosticsPage";
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
  const pages: PageId[] = ["status", "general", "board", "overlay"];
  if (settings.diagnostics) pages.push("diagnostics");
  // Diagnostics turned off while shown
  const shown = pages.includes(page) ? page : "status";

  return (
    <TextsContext.Provider value={texts}>
      <div className="app">
        <Sidebar pages={pages} page={shown} onSelect={setPage} />
        <main>
          <h1>{texts.nav[shown]}</h1>
          {shown === "status" && <StatusPage status={status} />}
          {shown === "board" && <BoardPage settings={settings} status={status} />}
          {shown === "overlay" && <OverlayPage settings={settings} status={status} />}
          {shown === "general" && <GeneralPage settings={settings} status={status} />}
          {shown === "diagnostics" && <DiagnosticsPage status={status} activity={activity} />}
        </main>
      </div>
    </TextsContext.Provider>
  );
}
