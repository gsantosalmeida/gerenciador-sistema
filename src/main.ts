import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";

type Settings = { downloadsPath: string; organize: boolean; tempMode: "none" | "age" | "all"; tempDays: number; logPath: string };
type Report = { lines: string[]; freedBytes: number; moved: number; deleted: number; skipped: number; errors: number };

const $ = <T extends HTMLElement>(selector: string) => document.querySelector<T>(selector)!;
const organize = $("#organize") as HTMLInputElement;
const tempAge = $("#temp-age") as HTMLInputElement;
const tempAll = $("#temp-all") as HTMLInputElement;
const tempDays = $("#temp-days") as HTMLInputElement;
const downloadsPath = $("#downloads-path") as HTMLInputElement;
const logPath = $("#log-path") as HTMLInputElement;
const simulate = $("#simulate") as HTMLInputElement;
const runButton = $("#run-button") as HTMLButtonElement;
const logOutput = $("#log-output");
let defaultDownloads = "";

function settings(): Settings {
  return {
    downloadsPath: downloadsPath.value.trim(),
    organize: organize.checked,
    tempMode: tempAll.checked ? "all" : tempAge.checked ? "age" : "none",
    tempDays: Math.max(0, Number(tempDays.value) || 0),
    logPath: logPath.value.trim(),
  };
}
function appendLines(lines: string[]) {
  if (logOutput.querySelector(".empty-state")) logOutput.innerHTML = "";
  const fragment = document.createDocumentFragment();
  for (const line of lines) {
    const row = document.createElement("div");
    row.textContent = line;
    row.className = line.includes("ERRO") ? "log-error" : line.includes("SIMULAÇÃO") ? "log-preview" : line.includes("APAGADO") || line.includes("MOVIDO") ? "log-success" : "";
    fragment.append(row);
  }
  logOutput.append(fragment);
  logOutput.scrollTop = logOutput.scrollHeight;
}
function formatBytes(bytes: number) {
  if (!bytes) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  return `${(bytes / 1024 ** i).toFixed(i ? 2 : 0)} ${units[i]}`;
}
function setBusy(busy: boolean) {
  runButton.disabled = busy;
  document.body.classList.toggle("is-busy", busy);
  $("#status-text").textContent = busy ? "Trabalhando…" : "Pronto para analisar";
  $("#log-count").textContent = busy ? "EM ANDAMENTO" : "PRONTO";
  $("#log-footer-text").textContent = busy ? "A operação está em andamento" : "Aguardando análise";
  runButton.innerHTML = busy ? "⟳ Processando…" : simulate.checked ? "⌕ Analisar" : "▶ Executar";
}
function syncMode() {
  tempDays.disabled = !tempAge.checked;
  $("#mode-hint").textContent = simulate.checked
    ? "A simulação mostra o que seria feito sem alterar arquivos."
    : "Ao executar, você verá uma confirmação antes de qualquer alteração.";
  runButton.innerHTML = simulate.checked ? "⌕ Analisar" : "▶ Executar";
}

async function run() {
  const config = settings();
  if (!config.organize && config.tempMode === "none") {
    appendLines(["Selecione ao menos uma ação para continuar."]);
    return;
  }
  if (!config.downloadsPath) {
    appendLines(["Informe a pasta Downloads antes de continuar."]);
    return;
  }
  if (!simulate.checked) {
    const warning = config.tempMode === "all"
      ? "A limpeza total tentará remover todos os arquivos e subpastas dentro das pastas TEMP selecionadas. Itens em uso serão mantidos. Deseja continuar?"
      : "Esta ação vai mover e/ou apagar arquivos no seu computador. Deseja continuar?";
    if (!window.confirm(warning)) return;
  }
  setBusy(true);
  document.body.classList.remove("is-error");
  logOutput.innerHTML = "";
  $("#freed-space").textContent = "Calculando…";
  try {
    const report = await invoke<Report>("run_cleaner", { settings: config, apply: !simulate.checked });
    appendLines(report.lines);
    $("#freed-space").textContent = `${formatBytes(report.freedBytes)}${simulate.checked ? " estimados" : " liberados"}`;
    $("#status-text").textContent = report.errors ? "Concluído com alguns avisos" : simulate.checked ? "Análise concluída" : "Limpeza concluída";
    $("#log-count").textContent = `${report.moved + report.deleted} ITENS`;
    $("#log-footer-text").textContent = report.errors ? `${report.errors} erro(s); consulte o registro` : "Operação concluída";
  } catch (error) {
    document.body.classList.add("is-error");
    const message = `Não foi possível completar a operação: ${String(error)}`;
    appendLines([message]);
    $("#freed-space").textContent = "Não calculado";
    $("#status-text").textContent = "A operação não foi concluída";
    $("#log-count").textContent = "ERRO";
    $("#log-footer-text").textContent = "Confira o erro acima";
  } finally { setBusy(false); }
}

window.addEventListener("DOMContentLoaded", async () => {
  try {
    defaultDownloads = await invoke<string>("default_downloads");
    downloadsPath.value = defaultDownloads;
  } catch {
    downloadsPath.value = "";
    appendLines(["Não foi possível detectar automaticamente a pasta Downloads. Informe o caminho manualmente."]);
  }
  runButton.addEventListener("click", run);
  simulate.addEventListener("change", syncMode);
  tempAge.addEventListener("change", syncMode);
  tempAll.addEventListener("change", syncMode);
  $("#choose-downloads").addEventListener("click", async () => {
    const selected = await open({ directory: true, multiple: false, defaultPath: downloadsPath.value || defaultDownloads, title: "Escolha a pasta Downloads" });
    if (typeof selected === "string") downloadsPath.value = selected;
  });
  $("#choose-log").addEventListener("click", async () => {
    const selected = await save({
      title: "Escolha onde salvar o registro",
      defaultPath: logPath.value || "registro-limpeza.log",
      filters: [{ name: "Registro de limpeza", extensions: ["log", "txt"] }],
    });
    if (selected) logPath.value = selected;
  });
  $("#clear-log-path").addEventListener("click", () => { logPath.value = ""; });
  $("#clear-log").addEventListener("click", () => {
    logOutput.innerHTML = '<div class="empty-state"><div class="empty-icon">✧</div><strong>Registro limpo</strong><p>Faça uma análise para ver a atividade aqui.</p></div>';
  });
  syncMode();
});
