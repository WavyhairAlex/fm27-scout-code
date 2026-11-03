import { Command, Database, Eye, Info, LockKeyhole, ShieldCheck, WifiOff } from "lucide-react";
import { useAppInfo } from "@/lib/query";
import { useI18n } from "@/lib/useI18n";
import { useAppStore } from "@/store/app";
import { Badge, Button, Modal } from "./ui";

export function HelpDialog() {
  const open = useAppStore((state) => state.helpOpen);
  const setOpen = useAppStore((state) => state.setHelpOpen);
  const { data: info } = useAppInfo(open);
  const { t } = useI18n();
  const shortcuts = [
    [t("shortcut.open"), t("help.shortcut.open")],
    [t("shortcut.search"), t("help.shortcut.search")],
    [t("shortcut.export"), t("help.shortcut.export")],
    [t("shortcut.settings"), t("help.shortcut.settings")],
    [t("shortcut.sort"), t("help.shortcut.sort")],
    [t("shortcut.escape"), t("help.shortcut.escape")],
  ];

  return (
    <Modal
      open={open}
      onClose={() => setOpen(false)}
      title={t("help.title")}
      description={t("help.description", {
        version: info?.version ?? __APP_VERSION__,
      })}
      width="max-w-[620px]"
      footer={
        <Button variant="secondary" size="sm" onClick={() => setOpen(false)}>
          {t("common.close")}
        </Button>
      }
    >
      <div className="grid gap-4 p-5 md:grid-cols-2">
        <section className="rounded-xl border border-[var(--line)] bg-[var(--surface-2)] p-3">
          <h3 className="flex items-center gap-2 text-[12px] font-semibold text-[var(--ink)]">
            <ShieldCheck size={14} className="text-[var(--accent)]" />
            {t("help.privacyTitle")}
          </h3>
          <ul className="mt-2 space-y-1.5 text-[10px] leading-relaxed text-[var(--muted)]">
            <li className="flex gap-1.5">
              <Check /> {t("help.privacy.1")}
            </li>
            <li className="flex gap-1.5">
              <Check /> {t("help.privacy.2")}
            </li>
            <li className="flex gap-1.5">
              <Check /> {t("help.privacy.3")}
            </li>
            <li className="flex gap-1.5">
              <Check /> {t("help.privacy.4")}
            </li>
          </ul>
        </section>

        <section className="rounded-xl border border-[var(--line)] bg-[var(--surface-2)] p-3">
          <h3 className="flex items-center gap-2 text-[12px] font-semibold text-[var(--ink)]">
            <Eye size={14} className="text-[var(--gold)]" />
            {t("help.hiddenTitle")}
          </h3>
          <p className="mt-2 text-[10px] leading-relaxed text-[var(--muted)]">
            {t("help.hiddenDescription")}
          </p>
          <div className="mt-2 flex gap-1.5">
            <Badge tone="danger">{t("help.soloOnly")}</Badge>
            <Badge tone="outline">{t("help.offByDefault")}</Badge>
          </div>
        </section>

        <section className="rounded-xl border border-[var(--line)] bg-[var(--surface-2)] p-3">
          <h3 className="flex items-center gap-2 text-[12px] font-semibold text-[var(--ink)]">
            <Command size={14} />
            {t("help.shortcuts")}
          </h3>
          <div className="mt-2 space-y-1">
            {shortcuts.map(([key, label]) => (
              <div key={key} className="flex items-center justify-between text-[10px]">
                <span className="text-[var(--muted)]">{label}</span>
                <span className="kbd">{key}</span>
              </div>
            ))}
          </div>
        </section>

        <section className="rounded-xl border border-[var(--line)] bg-[var(--surface-2)] p-3">
          <h3 className="flex items-center gap-2 text-[12px] font-semibold text-[var(--ink)]">
            <Info size={14} className="text-[var(--blue)]" />
            {t("help.parserTitle")}
          </h3>
          <p className="mt-2 text-[10px] leading-relaxed text-[var(--muted)]">
            {t("help.parserDescription")}
          </p>
          <div className="mt-2 flex flex-wrap gap-1.5">
            <Badge tone="outline">
              <Database size={9} /> {t("help.parserPlayers", { count: "46,273" })}
            </Badge>
            <Badge tone="outline">
              <LockKeyhole size={9} /> parser {info?.parser_version ?? __APP_VERSION__}
            </Badge>
            <Badge tone="outline">
              <WifiOff size={9} /> offline
            </Badge>
          </div>
        </section>

        <section className="md:col-span-2">
          <div className="rounded-xl border border-[var(--line)] px-3 py-2 text-[9px] leading-relaxed text-[var(--muted)]">
            {t("help.legal")}
          </div>
        </section>
      </div>
    </Modal>
  );
}

function Check() {
  return (
    <span className="mt-0.5 flex h-3 w-3 shrink-0 items-center justify-center rounded-full bg-[var(--accent-soft)] text-[8px] text-[var(--accent-strong)]">
      ✓
    </span>
  );
}
