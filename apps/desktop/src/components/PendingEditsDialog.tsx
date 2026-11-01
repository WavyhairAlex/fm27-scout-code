import { useState } from "react";
import {
  Check,
  ClipboardList,
  Loader2,
  Save,
  Trash2,
} from "lucide-react";
import { pickEditOutputFile, savePlayerEdits } from "@/lib/api";
import { attributeLabel, personalityLabel, traitLabel } from "@/lib/labels";
import { useOpenSave } from "@/lib/openSave";
import type { BatchEditResult } from "@/lib/types";
import { suggestedEditedSavePath } from "@/lib/utils";
import { useI18n } from "@/lib/useI18n";
import {
  useAppStore,
  type PendingChange,
  type PendingEdit,
} from "@/store/app";
import { Button, Modal } from "./ui";

export function PendingEditsDialog() {
  const { t, locale } = useI18n();
  const open = useAppStore((state) => state.pendingOpen);
  const setOpen = useAppStore((state) => state.setPendingOpen);
  const saveId = useAppStore((state) => state.saveId);
  const summary = useAppStore((state) => state.summary);
  const pendingEdits = useAppStore((state) => state.pendingEdits);
  const removePendingEdit = useAppStore((state) => state.removePendingEdit);
  const clearPendingEdits = useAppStore((state) => state.clearPendingEdits);
  const pushToast = useAppStore((state) => state.pushToast);
  const { openPath } = useOpenSave();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<BatchEditResult | null>(null);

  const entries = Object.values(pendingEdits).sort((left, right) =>
    left.name.localeCompare(right.name),
  );
  const totalChanges = entries.reduce((total, edit) => total + edit.changes.length, 0);

  const save = async () => {
    if (saveId == null || entries.length === 0) return;
    const output = await pickEditOutputFile(
      suggestedEditedSavePath(summary?.path, summary?.save_name),
    );
    if (!output) return;
    setBusy(true);
    setError(null);
    try {
      const written = await savePlayerEdits(
        saveId,
        output,
        entries.map((edit) => ({ uid: edit.uid, request: edit.request })),
      );
      setResult(written);
      clearPendingEdits();
      pushToast({
        title: t("toast.editDone"),
        description: written.output,
        tone: "success",
      });
    } catch (writeError) {
      const message = String(writeError);
      setError(message);
      pushToast({ title: t("toast.editFailed"), description: message, tone: "error" });
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal
      open={open}
      onClose={() => {
        if (!busy) setOpen(false);
      }}
      title={t("pending.title")}
      description={t("pending.description")}
      width="max-w-[720px]"
      footer={
        result ? (
          <>
            <Button variant="ghost" size="sm" onClick={() => setOpen(false)}>
              {t("common.close")}
            </Button>
            <Button
              variant="primary"
              size="sm"
              onClick={() => {
                setOpen(false);
                void openPath(result.output, true);
              }}
            >
              {t("pending.open")}
            </Button>
          </>
        ) : (
          <>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => {
                clearPendingEdits();
                setOpen(false);
              }}
              disabled={busy || entries.length === 0}
            >
              <Trash2 size={13} />
              {t("pending.removeAll")}
            </Button>
            <Button variant="ghost" size="sm" onClick={() => setOpen(false)} disabled={busy}>
              {t("common.cancel")}
            </Button>
            <Button
              variant="primary"
              size="sm"
              onClick={() => void save()}
              disabled={busy || entries.length === 0}
            >
              {busy ? <Loader2 size={13} className="animate-spin" /> : <Save size={13} />}
              {busy ? t("pending.saving") : t("pending.save")}
            </Button>
          </>
        )
      }
    >
      {result ? (
        <div className="flex flex-col items-center gap-3 px-6 py-12 text-center">
          <div className="flex h-12 w-12 items-center justify-center rounded-2xl border border-[var(--accent-line)] bg-[var(--accent-soft)] text-[var(--accent-strong)]">
            <Check size={22} />
          </div>
          <div className="text-sm font-semibold text-[var(--ink)]">
            {t("pending.successTitle")}
          </div>
          <div className="max-w-full break-all text-[11px] text-[var(--muted)]">
            {t("pending.successDescription", { path: result.output })}
          </div>
          <div className="text-[10px] text-[var(--muted)]">
            {t("pending.summary", {
              players: result.players,
              changes: result.changes,
            })}
          </div>
        </div>
      ) : entries.length === 0 ? (
        <div className="flex flex-col items-center gap-2 px-6 py-12 text-center">
          <ClipboardList size={22} className="text-[var(--muted)]" />
          <div className="text-xs font-semibold text-[var(--ink)]">{t("pending.empty")}</div>
        </div>
      ) : (
        <div className="space-y-3 p-5">
          <div className="flex items-center justify-between rounded-lg border border-[var(--line)] bg-[var(--surface-2)] px-3 py-2">
            <div className="flex items-center gap-2 text-[11px] text-[var(--muted)]">
              <ClipboardList size={14} className="text-[var(--accent)]" />
              {t("pending.summary", {
                players: entries.length,
                changes: totalChanges,
              })}
            </div>
            <span className="text-[10px] text-[var(--muted)]">{t("pending.hint")}</span>
          </div>

          <div className="space-y-2">
            {entries.map((edit) => (
              <PendingEditRow
                key={edit.uid}
                edit={edit}
                locale={locale}
                onRemove={() => removePendingEdit(edit.uid)}
              />
            ))}
          </div>

          {error && (
            <div className="rounded-lg border border-[var(--danger-line)] bg-[var(--danger-soft)] px-3 py-2 text-[11px] text-[var(--danger)]">
              {error}
            </div>
          )}
          <div className="rounded-lg border border-[var(--line)] bg-[var(--surface-2)] px-3 py-2 text-[10px] leading-relaxed text-[var(--muted)]">
            {t("pending.note")}
          </div>
        </div>
      )}
    </Modal>
  );
}

function PendingEditRow({
  edit,
  locale,
  onRemove,
}: {
  edit: PendingEdit;
  locale: "en" | "zh-CN";
  onRemove: () => void;
}) {
  const { t } = useI18n();
  return (
    <div className="rounded-xl border border-[var(--line)] bg-[var(--surface)] px-3 py-2.5">
      <div className="flex items-start justify-between gap-3">
        <div>
          <div className="text-[12px] font-semibold text-[var(--ink)]">{edit.name}</div>
          <div className="mt-0.5 text-[9px] text-[var(--muted)]">
            uid {edit.uid} · {t("pending.changeCount", { count: edit.changes.length })}
          </div>
        </div>
        <Button
          variant="ghost"
          size="sm"
          className="h-6 px-1.5 text-[10px]"
          onClick={onRemove}
        >
          <Trash2 size={11} />
          {t("pending.remove")}
        </Button>
      </div>
      <div className="mt-2 flex flex-wrap gap-1">
        {edit.changes.map((change, index) => (
          <span
            key={`${change.field}-${index}`}
            className="rounded-[4px] border border-[var(--line)] bg-[var(--surface-2)] px-1.5 py-0.5 text-[9px] text-[var(--muted)]"
          >
            {changeLabel(change, locale, t)}
          </span>
        ))}
      </div>
    </div>
  );
}

function changeLabel(
  change: PendingChange,
  locale: "en" | "zh-CN",
  t: (key: "pending.added" | "pending.removed") => string,
): string {
  if (change.field === "ca") return `CA ${change.before} → ${change.after}`;
  if (change.field === "pa") return `PA ${change.before} → ${change.after}`;
  if (change.field.startsWith("attribute.")) {
    const key = change.field.slice("attribute.".length);
    return `${attributeLabel(key, locale)} ${change.before} → ${change.after}`;
  }
  if (change.field.startsWith("personality.")) {
    const key = change.field.slice("personality.".length);
    return `${personalityLabel(key, locale)} ${change.before} → ${change.after}`;
  }
  if (change.field.startsWith("trait.")) {
    const bit = Number(change.field.slice("trait.".length));
    const label = traitLabel(bit, change.label ?? null, locale);
    return `${label} ${change.after > change.before ? t("pending.added") : t("pending.removed")}`;
  }
  return change.field;
}
