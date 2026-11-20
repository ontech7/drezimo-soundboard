import { invoke } from "@tauri-apps/api/core";
import { dirname, documentDir, join } from "@tauri-apps/api/path";
import { open, save } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";
import { Row, Section, Toggle } from "./controls";
import { type Texts, useTexts } from "./i18n";
import { Dialog } from "./pages/PadPicker";

/** What the checks left out of a file (src-tauri/src/backup.rs) */
interface Dropped {
  parts: number;
  pages: number;
  pads: number;
  macros: number;
  steps: number;
}

interface ImportPreview {
  exportedAt: string;
  appVersion: string;
  settings: boolean;
  pages: number | null;
  macros: number | null;
  display: boolean;
  dropped: Dropped;
}

interface Parts {
  settings: boolean;
  pages: boolean;
  macros: boolean;
  display: boolean;
}

interface ImportResult {
  cut: number;
  displaySkipped: boolean;
}

type ErrorKey = keyof Texts["backup"]["errors"];

/** What the card says after an action: worded when drawn, so that it follows a language the import changed */
type Message =
  | { kind: "exported"; name: string }
  | { kind: "imported" | "restored"; result: ImportResult }
  | { kind: "failed"; key: ErrorKey };

const dateFormat = new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" });

function today(): string {
  const now = new Date();
  const two = (number: number) => String(number).padStart(2, "0");
  return `${now.getFullYear()}-${two(now.getMonth() + 1)}-${two(now.getDate())}`;
}

function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

/**
 * The system's dialogs open in the folder used last, else in Documents: left to itself, Windows opens the app's working
 * folder, its program folder or a system one
 */
let lastFolder: string | undefined;

async function startFolder(): Promise<string | undefined> {
  return lastFolder ?? (await documentDir().catch(() => undefined));
}

async function keepFolder(path: string) {
  lastFolder = await dirname(path);
}

/** The commands fail with the key of a sentence; anything else, a system dialog's failure too, is a plain retry */
function failed(texts: Texts, error: unknown): Message {
  const key = typeof error === "string" && error in texts.backup.errors ? (error as ErrorKey) : "other";
  return { kind: "failed", key };
}

function lines(texts: Texts, message: Message): string[] {
  const backup = texts.backup;
  switch (message.kind) {
    case "exported":
      return [backup.exported(message.name)];
    case "failed":
      return [backup.errors[message.key]];
    case "imported":
    case "restored": {
      const { cut, displaySkipped } = message.result;
      return [
        message.kind === "imported" ? backup.imported : backup.restored,
        ...(cut > 0 ? [backup.cut(cut)] : []),
        ...(displaySkipped ? [backup.displaySkipped] : []),
      ];
    }
  }
}

function subtitle(texts: Texts, preview: ImportPreview): string | undefined {
  const at = new Date(preview.exportedAt);
  if (Number.isNaN(at.getTime())) return undefined;
  return texts.backup.exportedOn(dateFormat.format(at), preview.appVersion);
}

function droppedText(texts: Texts, dropped: Dropped): string | null {
  const words = texts.backup.dropped;
  const kinds = (Object.keys(words) as (keyof Dropped)[]).filter((kind) => dropped[kind] > 0);
  if (kinds.length === 0) return null;
  return `${texts.backup.droppedLead} ${kinds.map((kind) => words[kind](dropped[kind])).join(", ")}.`;
}

/** Export and import of the whole configuration, on the Settings page */
export function BackupCard() {
  const texts = useTexts();
  const backup = texts.backup;
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<Message | null>(null);
  const [preview, setPreview] = useState<ImportPreview | null>(null);
  const [parts, setParts] = useState<Parts>({ settings: false, pages: false, macros: false, display: false });
  const [canRestore, setCanRestore] = useState(false);

  const checkRestore = () => invoke<boolean>("has_before_import").then(setCanRestore);
  useEffect(() => {
    checkRestore();
  }, []);

  const run = async (action: () => Promise<Message | null>) => {
    setBusy(true);
    try {
      const shown = await action();
      if (shown) setMessage(shown);
    } finally {
      setBusy(false);
    }
  };

  const exportFile = () =>
    run(async () => {
      try {
        const filters = [{ name: backup.filter, extensions: ["json"] }];
        const folder = await startFolder();
        const name = backup.fileName(today());
        const path = await save({ defaultPath: folder ? await join(folder, name) : name, filters });
        if (!path) return null;
        await keepFolder(path);
        await invoke("export_configuration", { path });
        return { kind: "exported", name: fileName(path) };
      } catch (error) {
        return failed(texts, error === "cannotWrite" ? "cannotSave" : error);
      }
    });

  const importFile = () =>
    run(async () => {
      try {
        const filters = [{ name: backup.filter, extensions: ["json"] }];
        const path = await open({ multiple: false, directory: false, defaultPath: await startFolder(), filters });
        if (typeof path !== "string") return null;
        await keepFolder(path);
        const checked = await invoke<ImportPreview>("check_import", { path });
        setParts({
          settings: checked.settings,
          pages: checked.pages !== null,
          macros: checked.macros !== null,
          display: checked.display,
        });
        setPreview(checked);
        setMessage(null);
        return null;
      } catch (error) {
        return failed(texts, error);
      }
    });

  const cancel = () => {
    setPreview(null);
    invoke("cancel_import");
  };

  const confirm = () =>
    run(async () => {
      setPreview(null);
      try {
        const result = await invoke<ImportResult>("apply_import", { parts });
        return { kind: "imported", result };
      } catch (error) {
        return failed(texts, error);
      } finally {
        checkRestore();
      }
    });

  const restore = () =>
    run(async () => {
      try {
        const result = await invoke<ImportResult>("restore_before_import");
        return { kind: "restored", result };
      } catch (error) {
        return failed(texts, error);
      }
    });

  const part = (key: keyof Parts, label: string, text: string) => (
    <Row key={key} label={label} text={text}>
      <Toggle label={label} checked={parts[key]} onChange={(checked) => setParts({ ...parts, [key]: checked })} />
    </Row>
  );
  const usable = preview && (preview.settings || preview.pages !== null || preview.macros !== null || preview.display);
  const chosen = Object.values(parts).some(Boolean);
  const dropped = preview && droppedText(texts, preview.dropped);

  return (
    <>
      <Section title={backup.title}>
        <Row label={backup.exportLabel} text={backup.exportText}>
          <button type="button" className="button secondary" disabled={busy} onClick={exportFile}>
            {backup.exportButton}
          </button>
        </Row>
        <Row label={backup.importLabel} text={backup.importText}>
          <button type="button" className="button secondary" disabled={busy} onClick={importFile}>
            {backup.importButton}
          </button>
        </Row>
        {canRestore && (
          <Row label={backup.restoreLabel} text={backup.restoreText}>
            <button type="button" className="button secondary" disabled={busy} onClick={restore}>
              {backup.restoreButton}
            </button>
          </Row>
        )}
        {message && (
          <div className={message.kind === "failed" ? "backup-message failed" : "backup-message"} role="status">
            {lines(texts, message).map((line) => (
              <p key={line}>{line}</p>
            ))}
          </div>
        )}
      </Section>

      {preview && (
        <Dialog
          label={backup.dialogTitle}
          title={backup.dialogTitle}
          subtitle={subtitle(texts, preview)}
          className="compact"
          onClose={cancel}
        >
          <div className="dialog-body">
            {usable ? (
              <div className="rows">
                {preview.settings && part("settings", backup.settings, backup.settingsText)}
                {preview.pages !== null && part("pages", backup.pages(preview.pages), backup.pagesText)}
                {preview.macros !== null && part("macros", backup.macros(preview.macros), backup.macrosText)}
                {preview.display && part("display", backup.display, backup.displayText)}
              </div>
            ) : (
              <p className="muted">{backup.nothingUsable}</p>
            )}
            {dropped && <p className="backup-dropped">{dropped}</p>}
          </div>
          <footer>
            <p className="muted">{usable ? backup.copyNote : ""}</p>
            <button type="button" className="button secondary" onClick={cancel}>
              {texts.cancel}
            </button>
            <button type="button" className="button" data-autofocus disabled={!chosen} onClick={confirm}>
              {backup.confirm}
            </button>
          </footer>
        </Dialog>
      )}
    </>
  );
}
