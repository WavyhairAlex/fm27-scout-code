import { useCallback } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { confirm as confirmDialog } from "@tauri-apps/plugin-dialog";
import {
  isTauri,
  listenParseProgress,
  logMessage,
  openSave,
  pickSaveFile,
} from "./api";
import { formatNumber } from "./i18n";
import { useI18n } from "./useI18n";
import { useAppStore } from "@/store/app";

export function useOpenSave() {
  const { t, locale } = useI18n();
  const setSession = useAppStore((state) => state.setSession);
  const setParsing = useAppStore((state) => state.setParsing);
  const setParseProgress = useAppStore((state) => state.setParseProgress);
  const pushToast = useAppStore((state) => state.pushToast);
  const queryClient = useQueryClient();

  const openPath = useCallback(
    async (path: string, force = false) => {
      const pendingCount = Object.keys(useAppStore.getState().pendingEdits).length;
      if (pendingCount > 0) {
        const message = t("pending.discardBeforeOpen", { count: pendingCount });
        const confirmed = isTauri
          ? await confirmDialog(message, {
              title: t("pending.title"),
              kind: "warning",
            })
          : window.confirm(message);
        if (!confirmed) return;
      }
      setParsing(true);
      setParseProgress({ stage: "opening", current: 0, total: 0, message: "Opening save" });
      let unlisten: () => void = () => undefined;
      try {
        unlisten = await listenParseProgress((progress) => setParseProgress(progress));
        const result = await openSave(path, force);
        setSession(result.save_id, result.summary);
        await queryClient.invalidateQueries();
        pushToast({
          title: t(result.cached ? "toast.cachedSave" : "toast.parseDone"),
          description: result.cached
            ? t("toast.cacheDescription", { name: result.summary.save_name })
            : t("toast.parsedDescription", {
                count: formatNumber(result.report?.players ?? 0, locale),
                seconds: ((result.report?.elapsed_ms ?? 0) / 1000).toFixed(2),
              }),
          tone: "success",
        });
      } catch (error) {
        await logMessage(`openSave failed: ${String(error)}`);
        pushToast({
          title: t("toast.openFailed"),
          description: String(error),
          tone: "error",
        });
      } finally {
        unlisten();
        setParsing(false);
      }
    },
    [locale, pushToast, queryClient, setParseProgress, setParsing, setSession, t],
  );

  const chooseAndOpen = useCallback(async () => {
    const path = await pickSaveFile(t("dialog.saveFilter"));
    if (path) await openPath(path);
  }, [openPath, t]);

  return { openPath, chooseAndOpen };
}
