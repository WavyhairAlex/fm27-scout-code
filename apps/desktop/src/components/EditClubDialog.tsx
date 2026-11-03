import { useEffect, useMemo, useState } from "react";
import { AlertTriangle, Loader2, Save } from "lucide-react";
import { editClub, pickEditOutputFile } from "@/lib/api";
import { facilityRatingLabel } from "@/lib/labels";
import { useClubDetail } from "@/lib/query";
import type { ClubEditRequest, ClubFinanceEdit, StadiumCapacityEdit } from "@/lib/types";
import { suggestedEditedSavePath } from "@/lib/utils";
import { useI18n } from "@/lib/useI18n";
import { useAppStore } from "@/store/app";
import { Button, Input, Modal } from "./ui";

interface FinanceForm {
  balance: number | null;
  transferBudget: number | null;
  wageBudget: number | null;
}

interface StadiumForm {
  allSeater: number | null;
  expansion: number | null;
}

export function EditClubDialog() {
  const { t, locale } = useI18n();
  const open = useAppStore((state) => state.clubEditOpen);
  const setOpen = useAppStore((state) => state.setClubEditOpen);
  const saveId = useAppStore((state) => state.saveId);
  const summary = useAppStore((state) => state.summary);
  const selectedClubUid = useAppStore((state) => state.selectedClubUid);
  const pushToast = useAppStore((state) => state.pushToast);
  const { data: detail } = useClubDetail(open ? selectedClubUid : null);

  const [reputation, setReputation] = useState<number | null>(null);
  const [corporate, setCorporate] = useState<number | null>(null);
  const [finance, setFinance] = useState<FinanceForm>({
    balance: null,
    transferBudget: null,
    wageBudget: null,
  });
  const [stadiums, setStadiums] = useState<Record<number, StadiumForm>>({});
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const latestFinance = detail?.finance_months.at(-1) ?? null;

  useEffect(() => {
    if (!open || !detail) return;
    setReputation(detail.club.reputation);
    setCorporate(detail.club.corporate_facilities);
    setFinance({
      balance: latestFinance?.balance ?? null,
      transferBudget: latestFinance?.transfer_budget_remaining ?? null,
      wageBudget: latestFinance?.wage_budget_weekly ?? null,
    });
    setStadiums(
      Object.fromEntries(
        detail.stadiums.map((stadium) => [
          stadium.uid,
          {
            allSeater: stadium.all_seater_capacity,
            expansion: stadium.expansion_capacity,
          },
        ]),
      ),
    );
    setError(null);
    setBusy(false);
  }, [open, detail?.club.uid, latestFinance?.month]);

  const request = useMemo<ClubEditRequest | null>(() => {
    if (!detail) return null;
    const club = detail.club;
    const financeEdit: ClubFinanceEdit = {
      balance:
        finance.balance != null && finance.balance !== (latestFinance?.balance ?? null)
          ? finance.balance
          : null,
      transfer_budget_remaining:
        finance.transferBudget != null &&
        finance.transferBudget !== (latestFinance?.transfer_budget_remaining ?? null)
          ? finance.transferBudget
          : null,
      wage_budget_weekly:
        finance.wageBudget != null &&
        finance.wageBudget !== (latestFinance?.wage_budget_weekly ?? null)
          ? finance.wageBudget
          : null,
    };
    const stadiumEdits: StadiumCapacityEdit[] = [];
    for (const stadium of detail.stadiums) {
      const form = stadiums[stadium.uid];
      if (!form) continue;
      const entry: StadiumCapacityEdit = {
        stadium_uid: stadium.uid,
        all_seater_capacity:
          form.allSeater != null && form.allSeater !== stadium.all_seater_capacity
            ? form.allSeater
            : null,
        expansion_capacity:
          form.expansion != null && form.expansion !== stadium.expansion_capacity
            ? form.expansion
            : null,
      };
      if (entry.all_seater_capacity != null || entry.expansion_capacity != null) {
        stadiumEdits.push(entry);
      }
    }
    const hasFinance =
      financeEdit.balance != null ||
      financeEdit.transfer_budget_remaining != null ||
      financeEdit.wage_budget_weekly != null;
    return {
      reputation:
        reputation != null && reputation !== club.reputation ? reputation : null,
      corporate_facilities:
        corporate != null && corporate !== club.corporate_facilities ? corporate : null,
      finances: hasFinance ? financeEdit : null,
      stadiums: stadiumEdits,
    };
  }, [detail, finance, latestFinance, reputation, corporate, stadiums]);

  const hasChanges =
    request != null &&
    (request.reputation != null ||
      request.corporate_facilities != null ||
      request.finances != null ||
      request.stadiums.length > 0);

  const save = async () => {
    if (saveId == null || selectedClubUid == null || request == null || !hasChanges) return;
    const output = await pickEditOutputFile(
      suggestedEditedSavePath(summary?.path, summary?.save_name),
    );
    if (!output) return;
    setBusy(true);
    setError(null);
    try {
      const result = await editClub(saveId, selectedClubUid, output, request);
      pushToast({
        title: t("clubs.edit.done"),
        description: result.output,
        tone: "success",
      });
      setOpen(false);
    } catch (writeError) {
      const message = String(writeError);
      setError(message);
      pushToast({ title: t("clubs.edit.failed"), description: message, tone: "error" });
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
      title={t("clubs.edit.title")}
      description={
        detail ? `${detail.club.name} · ${t("clubs.edit.description")}` : t("clubs.edit.description")
      }
      width="max-w-[820px]"
      footer={
        <>
          <Button variant="ghost" size="sm" onClick={() => setOpen(false)} disabled={busy}>
            {t("common.cancel")}
          </Button>
          <Button
            variant="primary"
            size="sm"
            onClick={() => void save()}
            disabled={busy || !hasChanges}
          >
            {busy ? <Loader2 size={13} className="animate-spin" /> : <Save size={13} />}
            {busy ? t("clubs.edit.saving") : t("clubs.edit.save")}
          </Button>
        </>
      }
    >
      {!detail ? (
        <div className="flex items-center justify-center py-10 text-[11px] text-[var(--muted)]">
          <Loader2 size={14} className="mr-2 animate-spin" />
          {t("clubs.loading")}
        </div>
      ) : (
        <div className="space-y-4 p-4">
          {error && (
            <div className="rounded-xl border border-[var(--danger-line)] bg-[var(--danger-soft)] px-3 py-2 text-[11px] text-[var(--danger)]">
              {error}
            </div>
          )}
          <div className="flex items-start gap-2 rounded-xl border border-[var(--gold-line)] bg-[var(--gold-soft)] px-3 py-2 text-[10px] leading-relaxed text-[var(--gold)]">
            <AlertTriangle size={13} className="mt-0.5 shrink-0" />
            <span>{t("clubs.edit.warning")}</span>
          </div>

          <Section title={t("clubs.edit.club")}>
            <Field
              label={t("clubs.edit.reputation")}
              value={reputation}
              min={1}
              max={10000}
              onChange={setReputation}
            />
            <Field
              label={t("clubs.edit.corporateFacilities")}
              value={corporate}
              min={1}
              max={20}
              disabled={latestFinance == null}
              hint={
                latestFinance == null
                  ? t("clubs.edit.unavailable")
                  : (facilityRatingLabel(corporate, locale) ?? undefined)
              }
              onChange={setCorporate}
            />
          </Section>

          <Section title={t("clubs.edit.finances")}>
            {latestFinance == null ? (
              <div className="col-span-2 text-[10px] text-[var(--muted)]">
                {t("clubs.inspector.noFinance")}
              </div>
            ) : (
              <>
                <Field
                  label={t("clubs.inspector.balance")}
                  value={finance.balance}
                  onChange={(value) => setFinance((current) => ({ ...current, balance: value }))}
                />
                <Field
                  label={t("clubs.inspector.transferBudget")}
                  value={finance.transferBudget}
                  onChange={(value) =>
                    setFinance((current) => ({ ...current, transferBudget: value }))
                  }
                />
                <Field
                  label={t("clubs.inspector.wageBudget")}
                  value={finance.wageBudget}
                  min={0}
                  onChange={(value) =>
                    setFinance((current) => ({ ...current, wageBudget: value }))
                  }
                />
              </>
            )}
          </Section>

          <Section title={t("clubs.edit.stadiums")}>
            {detail.stadiums.length === 0 ? (
              <div className="col-span-2 text-[10px] text-[var(--muted)]">
                {t("clubs.inspector.noStadium")}
              </div>
            ) : (
              detail.stadiums.map((stadium) => (
                <div
                  key={stadium.uid}
                  className="col-span-2 rounded-xl border border-[var(--line)] bg-[var(--surface-2)] p-2.5"
                >
                  <div className="mb-2 flex items-center justify-between gap-2">
                    <span className="truncate text-[11px] font-semibold text-[var(--ink)]">
                      {stadium.name ?? t("clubs.inspector.unnamedStadium")}
                    </span>
                  </div>
                  <div className="grid grid-cols-2 gap-2">
                    <Field
                      label={t("clubs.edit.allSeater")}
                      value={stadiums[stadium.uid]?.allSeater ?? null}
                      min={0}
                      max={10_000_000}
                      onChange={(value) =>
                        setStadiums((current) => ({
                          ...current,
                          [stadium.uid]: {
                            ...(current[stadium.uid] ?? {
                              allSeater: stadium.all_seater_capacity,
                              expansion: stadium.expansion_capacity,
                            }),
                            allSeater: value,
                          },
                        }))
                      }
                    />
                    <Field
                      label={t("clubs.edit.expansion")}
                      value={stadiums[stadium.uid]?.expansion ?? null}
                      min={0}
                      max={10_000_000}
                      onChange={(value) =>
                        setStadiums((current) => ({
                          ...current,
                          [stadium.uid]: {
                            ...(current[stadium.uid] ?? {
                              allSeater: stadium.all_seater_capacity,
                              expansion: stadium.expansion_capacity,
                            }),
                            expansion: value,
                          },
                        }))
                      }
                    />
                  </div>
                </div>
              ))
            )}
          </Section>
        </div>
      )}
    </Modal>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section>
      <div className="mb-1.5 text-[10px] font-semibold uppercase tracking-wider text-[var(--muted)]">
        {title}
      </div>
      <div className="grid grid-cols-2 gap-2">{children}</div>
    </section>
  );
}

function Field({
  label,
  value,
  onChange,
  min,
  max,
  disabled,
  hint,
}: {
  label: string;
  value: number | null;
  onChange: (value: number | null) => void;
  min?: number;
  max?: number;
  disabled?: boolean;
  hint?: string;
}) {
  return (
    <label className="space-y-1">
      <span className="block text-[9px] uppercase tracking-wider text-[var(--muted)]">
        {label}
      </span>
      <Input
        type="number"
        value={value ?? ""}
        min={min}
        max={max}
        disabled={disabled}
        onChange={(event) => {
          const parsed = Number(event.target.value);
          onChange(event.target.value === "" || !Number.isFinite(parsed) ? null : parsed);
        }}
        className="h-8 font-mono"
      />
      {hint && <span className="block text-[8px] text-[var(--muted)]">{hint}</span>}
    </label>
  );
}
