import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { FilterSidebar } from "./components/FilterSidebar";
import { PlayerTable } from "./components/PlayerTable";
import { ClubsView } from "./components/ClubsView";
import { StaffView } from "./components/StaffView";
import { Inspector } from "./components/Inspector";
import { Toolbar } from "./components/Toolbar";
import { Onboarding } from "./components/Onboarding";
import { CompareDialog } from "./components/CompareDialog";
import { AnalysisDialog } from "./components/AnalysisDialog";
import { SettingsDialog } from "./components/SettingsDialog";
import { ExportDialog } from "./components/ExportDialog";
import { HelpDialog } from "./components/HelpDialog";
import { EditPlayerDialog } from "./components/EditPlayerDialog";
import { PendingEditsDialog } from "./components/PendingEditsDialog";
import { ParseOverlay } from "./components/ParseOverlay";
import { Toasts } from "./components/Toasts";
import { useOpenSave } from "./lib/openSave";
import {
  listenMenuAction,
  listenOpenFile,
  logMessage,
  setAppLanguage,
  takePendingOpen,
} from "./lib/api";
import { useI18n } from "./lib/useI18n";
import { useAppStore } from "./store/app";

export default function App() {
  const saveId = useAppStore((state) => state.saveId);
  const viewMode = useAppStore((state) => state.viewMode);
  const sidebarOpen = useAppStore((state) => state.sidebarOpen);
  const inspectorOpen = useAppStore((state) => state.inspectorOpen);
  const selectedUid = useAppStore((state) => state.selectedUid);
  const selectedClubUid = useAppStore((state) => state.selectedClubUid);
  const selectedStaffUid = useAppStore((state) => state.selectedStaffUid);
  const setSelectedUid = useAppStore((state) => state.setSelectedUid);
  const setSelectedClubUid = useAppStore((state) => state.setSelectedClubUid);
  const setSelectedStaffUid = useAppStore((state) => state.setSelectedStaffUid);
  const setExportOpen = useAppStore((state) => state.setExportOpen);
  const setEditOpen = useAppStore((state) => state.setEditOpen);
  const setPendingOpen = useAppStore((state) => state.setPendingOpen);
  const setSettingsOpen = useAppStore((state) => state.setSettingsOpen);
  const setCompareOpen = useAppStore((state) => state.setCompareOpen);
  const setAnalysisOpen = useAppStore((state) => state.setAnalysisOpen);
  const setHelpOpen = useAppStore((state) => state.setHelpOpen);
  const compareOpen = useAppStore((state) => state.compareOpen);
  const settingsOpen = useAppStore((state) => state.settingsOpen);
  const exportOpen = useAppStore((state) => state.exportOpen);
  const editOpen = useAppStore((state) => state.editOpen);
  const pendingOpen = useAppStore((state) => state.pendingOpen);
  const helpOpen = useAppStore((state) => state.helpOpen);
  const theme = useAppStore((state) => state.theme);
  const setSidebarOpen = useAppStore((state) => state.setSidebarOpen);
  const setInspectorOpen = useAppStore((state) => state.setInspectorOpen);
  const { chooseAndOpen, openPath } = useOpenSave();
  const { locale } = useI18n();
  const queryClient = useQueryClient();

  useEffect(() => {
    document.documentElement.lang = locale;
    void setAppLanguage(locale);
  }, [locale]);

  useEffect(() => {
    const root = document.documentElement;
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      const dark = theme === "dark" || (theme === "system" && media.matches);
      root.classList.toggle("dark", dark);
      root.style.colorScheme = dark ? "dark" : "light";
    };
    apply();
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  }, [theme]);

  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      const meta = event.metaKey || event.ctrlKey;
      if (meta && event.key.toLowerCase() === "o") {
        event.preventDefault();
        void chooseAndOpen();
        return;
      }
      if (meta && event.key.toLowerCase() === "f") {
        event.preventDefault();
        document
          .getElementById(
            viewMode === "clubs"
              ? "club-search"
              : viewMode === "staff"
                ? "staff-search"
                : "global-search",
          )
          ?.focus();
        return;
      }
      if (meta && event.key.toLowerCase() === "e") {
        event.preventDefault();
        setExportOpen(true);
        return;
      }
      if (meta && event.key === ",") {
        event.preventDefault();
        setSettingsOpen(true);
        return;
      }
      if (event.key === "Escape") {
        if (compareOpen) setCompareOpen(false);
        else if (settingsOpen) setSettingsOpen(false);
        else if (exportOpen) setExportOpen(false);
        else if (editOpen) setEditOpen(false);
        else if (pendingOpen) setPendingOpen(false);
        else if (helpOpen) setHelpOpen(false);
        else if (viewMode === "clubs" && selectedClubUid != null) setSelectedClubUid(null);
        else if (viewMode === "staff" && selectedStaffUid != null) setSelectedStaffUid(null);
        else if (selectedUid != null) setSelectedUid(null);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [
    chooseAndOpen,
    compareOpen,
    editOpen,
    exportOpen,
    helpOpen,
    pendingOpen,
    selectedClubUid,
    selectedStaffUid,
    selectedUid,
    setCompareOpen,
    setEditOpen,
    setExportOpen,
    setPendingOpen,
    setHelpOpen,
    setSelectedClubUid,
    setSelectedStaffUid,
    setSelectedUid,
    setSettingsOpen,
    settingsOpen,
    viewMode,
  ]);

  useEffect(() => {
    void queryClient.invalidateQueries({ queryKey: ["players"] });
  }, [queryClient]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let timer: number | undefined;
    const consumePending = async () => {
      try {
        const path = await takePendingOpen();
        if (path?.toLowerCase().endsWith(".fm")) {
          if (timer) window.clearInterval(timer);
          await openPath(path);
          return true;
        }
      } catch (error) {
        await logMessage(`takePendingOpen error: ${String(error)}`);
      }
      return false;
    };
    let attempts = 0;
    timer = window.setInterval(() => {
      attempts += 1;
      void consumePending();
      if (attempts >= 12) window.clearInterval(timer);
    }, 1000);
    void consumePending();
    void listenOpenFile(() => {
      void consumePending();
    }).then((cleanup) => {
      unlisten = cleanup;
    });
    return () => {
      window.clearInterval(timer);
      unlisten?.();
    };
  }, [openPath]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listenMenuAction((action) => {
      switch (action) {
        case "open":
          void chooseAndOpen();
          break;
        case "export":
          setExportOpen(true);
          break;
        case "settings":
          setSettingsOpen(true);
          break;
        case "sidebar":
          setSidebarOpen(!useAppStore.getState().sidebarOpen);
          break;
        case "inspector":
          setInspectorOpen(!useAppStore.getState().inspectorOpen);
          break;
        case "compare":
          if (useAppStore.getState().compareUids.length >= 2) setCompareOpen(true);
          break;
        case "analysis":
          setAnalysisOpen(true);
          break;
        case "help":
          setHelpOpen(true);
          break;
      }
    }).then((cleanup) => {
      unlisten = cleanup;
    });
    return () => unlisten?.();
  }, [
    chooseAndOpen,
    setAnalysisOpen,
    setCompareOpen,
    setExportOpen,
    setHelpOpen,
    setInspectorOpen,
    setSettingsOpen,
    setSidebarOpen,
  ]);

  return (
    <div className="app-bg flex h-full flex-col">
      <Toolbar />
      <div className="flex min-h-0 flex-1">
        {saveId == null ? (
          <Onboarding />
        ) : viewMode === "clubs" ? (
          <ClubsView />
        ) : viewMode === "staff" ? (
          <StaffView />
        ) : (
          <>
            {sidebarOpen && <FilterSidebar />}
            <PlayerTable />
            {inspectorOpen && <Inspector />}
          </>
        )}
      </div>
      <CompareDialog />
      <AnalysisDialog />
      <SettingsDialog />
      <ExportDialog />
      <EditPlayerDialog />
      <PendingEditsDialog />
      <HelpDialog />
      <ParseOverlay />
      <Toasts />
    </div>
  );
}
