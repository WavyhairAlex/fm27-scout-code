import { useEffect, useRef, useState } from "react";
import {
  BarChart3,
  BriefcaseBusiness,
  ChevronDown,
  ClipboardList,
  Database,
  Download,
  FilePlus2,
  FolderOpen,
  GitCompareArrows,
  HelpCircle,
  PanelLeftClose,
  PanelLeftOpen,
  Search,
  Settings2,
  Shield,
  Users,
  X,
} from "lucide-react";
import { useSaves } from "@/lib/query";
import { relativeTime, cn } from "@/lib/utils";
import { formatNumber } from "@/lib/i18n";
import { useI18n } from "@/lib/useI18n";
import { useAppStore, type DataMode } from "@/store/app";
import { useOpenSave } from "@/lib/openSave";
import { Badge, Button, Input, Select } from "./ui";

export function Toolbar() {
  const { t, locale } = useI18n();
  const summary = useAppStore((state) => state.summary);
  const viewMode = useAppStore((state) => state.viewMode);
  const setViewMode = useAppStore((state) => state.setViewMode);
  const sidebarOpen = useAppStore((state) => state.sidebarOpen);
  const setSidebarOpen = useAppStore((state) => state.setSidebarOpen);
  const filters = useAppStore((state) => state.filters);
  const patchFilters = useAppStore((state) => state.patchFilters);
  const clubFilters = useAppStore((state) => state.clubFilters);
  const patchClubFilters = useAppStore((state) => state.patchClubFilters);
  const staffFilters = useAppStore((state) => state.staffFilters);
  const patchStaffFilters = useAppStore((state) => state.patchStaffFilters);
  const dataMode = useAppStore((state) => state.dataMode);
  const setDataMode = useAppStore((state) => state.setDataMode);
  const compareUids = useAppStore((state) => state.compareUids);
  const setCompareOpen = useAppStore((state) => state.setCompareOpen);
  const setAnalysisOpen = useAppStore((state) => state.setAnalysisOpen);
  const setExportOpen = useAppStore((state) => state.setExportOpen);
  const setPendingOpen = useAppStore((state) => state.setPendingOpen);
  const pendingEdits = useAppStore((state) => state.pendingEdits);
  const setSettingsOpen = useAppStore((state) => state.setSettingsOpen);
  const setHelpOpen = useAppStore((state) => state.setHelpOpen);
  const { chooseAndOpen, openPath } = useOpenSave();
  const { data: saves = [] } = useSaves();
  const [saveMenuOpen, setSaveMenuOpen] = useState(false);
  const searchValue =
    viewMode === "clubs"
      ? clubFilters.search
      : viewMode === "staff"
        ? staffFilters.search
        : filters.search;
  const [search, setSearch] = useState(searchValue);
  const saveMenuRef = useRef<HTMLDivElement>(null);
  const debounceRef = useRef<number | null>(null);

  useEffect(() => {
    setSearch(searchValue);
  }, [searchValue]);

  useEffect(() => {
    if (!saveMenuOpen) return;
    const handler = (event: MouseEvent) => {
      if (!saveMenuRef.current?.contains(event.target as Node)) setSaveMenuOpen(false);
    };
    window.addEventListener("mousedown", handler);
    return () => window.removeEventListener("mousedown", handler);
  }, [saveMenuOpen]);

  const updateSearch = (value: string) => {
    setSearch(value);
    if (debounceRef.current) window.clearTimeout(debounceRef.current);
    debounceRef.current = window.setTimeout(() => {
      if (viewMode === "clubs") patchClubFilters({ search: value });
      else if (viewMode === "staff") patchStaffFilters({ search: value });
      else patchFilters({ search: value });
    }, 180);
  };

  return (
    <header className="titlebar-drag relative z-40 flex h-11 shrink-0 items-center gap-2 border-b border-[var(--line)] bg-[var(--surface)]/95 px-3 pl-[86px] backdrop-blur">
      <div className="no-drag flex items-center gap-2">
        <Button
          variant="ghost"
          size="icon"
          className="h-7 w-7"
          onClick={() => setSidebarOpen(!sidebarOpen)}
          aria-label={sidebarOpen ? t("toolbar.hideSidebar") : t("toolbar.showSidebar")}
          title={sidebarOpen ? t("toolbar.hideSidebar") : t("toolbar.showSidebar")}
        >
          {sidebarOpen ? <PanelLeftClose size={15} /> : <PanelLeftOpen size={15} />}
        </Button>

        <div ref={saveMenuRef} className="relative">
          <button
            type="button"
            onClick={() => setSaveMenuOpen((value) => !value)}
            className="focus-ring flex h-8 items-center gap-2 rounded-[7px] border border-[var(--line)] bg-[var(--surface-2)] px-2.5 text-left transition-colors hover:border-[var(--line-strong)]"
          >
            <Database size={13} className="text-[var(--accent)]" />
            <span className="max-w-[220px] truncate text-[11px] font-semibold text-[var(--ink)]">
              {summary?.save_name ?? t("toolbar.noSave")}
            </span>
            {summary?.game_date && (
              <span className="font-mono text-[10px] text-[var(--muted)]">{summary.game_date}</span>
            )}
            <ChevronDown size={12} className="text-[var(--muted)]" />
          </button>
          {saveMenuOpen && (
            <div className="absolute left-0 top-[calc(100%+5px)] z-40 w-[360px] overflow-hidden rounded-xl border border-[var(--line)] bg-[var(--surface)] shadow-[var(--shadow-pop)]">
              <div className="border-b border-[var(--line)] px-3 py-2">
                <div className="text-[10px] font-semibold uppercase tracking-wider text-[var(--muted)]">
                  {t("toolbar.recentSaves")}
                </div>
              </div>
              <div className="max-h-[320px] overflow-auto p-1.5">
                {saves.length === 0 && (
                  <div className="px-2 py-4 text-center text-[11px] text-[var(--muted)]">
                    {t("toolbar.noCachedSaves")}
                  </div>
                )}
                {saves.map((save) => (
                  <button
                    key={save.id}
                    type="button"
                    onClick={() => {
                      setSaveMenuOpen(false);
                      void openPath(save.path);
                    }}
                    className="flex w-full items-start gap-2 rounded-[8px] px-2 py-2 text-left hover:bg-[var(--surface-2)]"
                  >
                    <div className="mt-0.5 flex h-7 w-7 shrink-0 items-center justify-center rounded-lg border border-[var(--line)] bg-[var(--surface-2)] text-[var(--accent)]">
                      <Database size={13} />
                    </div>
                    <div className="min-w-0 flex-1">
                      <div className="truncate text-[11px] font-semibold text-[var(--ink)]">
                        {save.file_name}
                      </div>
                      <div className="mt-0.5 flex items-center gap-2 text-[9px] text-[var(--muted)]">
                        <span>{save.game_date ?? t("common.unknownDate")}</span>
                        <span>·</span>
                        <span>
                          {t("toolbar.people", {
                            count: formatNumber(save.players, locale),
                          })}
                        </span>
                        <span>·</span>
                        <span>{relativeTime(save.parsed_at, locale)}</span>
                      </div>
                    </div>
                    {summary?.path === save.path && (
                      <Badge tone="accent">{t("toolbar.current")}</Badge>
                    )}
                  </button>
                ))}
              </div>
              <div className="border-t border-[var(--line)] p-1.5">
                <button
                  type="button"
                  onClick={() => {
                    setSaveMenuOpen(false);
                    void chooseAndOpen();
                  }}
                  className="flex w-full items-center gap-2 rounded-[7px] px-2 py-2 text-[11px] font-medium text-[var(--accent-strong)] hover:bg-[var(--accent-soft)]"
                >
                  <FolderOpen size={13} />
                  {t("toolbar.openOther")}
                  <span className="ml-auto kbd">⌘O</span>
                </button>
              </div>
            </div>
          )}
        </div>

        <Button variant="secondary" size="sm" onClick={() => void chooseAndOpen()} className="h-8">
          <FilePlus2 size={13} />
          {t("toolbar.open")}
        </Button>

        <div className="flex h-8 items-center rounded-[7px] border border-[var(--line)] bg-[var(--surface-2)] p-0.5">
          <button
            type="button"
            onClick={() => setViewMode("players")}
            className={cn(
              "focus-ring flex h-6 items-center gap-1 rounded-[5px] px-2 text-[10px] font-medium transition-colors",
              viewMode === "players"
                ? "bg-[var(--surface)] text-[var(--ink)] shadow-sm"
                : "text-[var(--muted)] hover:text-[var(--ink)]",
            )}
          >
            <Users size={11} />
            {t("toolbar.players")}
          </button>
          <button
            type="button"
            onClick={() => setViewMode("clubs")}
            className={cn(
              "focus-ring flex h-6 items-center gap-1 rounded-[5px] px-2 text-[10px] font-medium transition-colors",
              viewMode === "clubs"
                ? "bg-[var(--surface)] text-[var(--ink)] shadow-sm"
                : "text-[var(--muted)] hover:text-[var(--ink)]",
            )}
          >
            <Shield size={11} />
            {t("toolbar.clubs")}
          </button>
          <button
            type="button"
            onClick={() => setViewMode("staff")}
            className={cn(
              "focus-ring flex h-6 items-center gap-1 rounded-[5px] px-2 text-[10px] font-medium transition-colors",
              viewMode === "staff"
                ? "bg-[var(--surface)] text-[var(--ink)] shadow-sm"
                : "text-[var(--muted)] hover:text-[var(--ink)]",
            )}
          >
            <BriefcaseBusiness size={11} />
            {t("toolbar.staff")}
          </button>
        </div>
      </div>

      <div className="no-drag flex min-w-0 flex-1 items-center justify-center">
        <div className="relative w-full max-w-[560px]">
          <Search
            size={14}
            className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-[var(--muted)]"
          />
          <Input
            id={
              viewMode === "clubs"
                ? "club-search"
                : viewMode === "staff"
                  ? "staff-search"
                  : "global-search"
            }
            value={search}
            onChange={(event) => updateSearch(event.target.value)}
            placeholder={
              viewMode === "clubs"
                ? t("clubs.searchPlaceholder")
                : viewMode === "staff"
                  ? t("toolbar.searchStaffPlaceholder")
                  : t("toolbar.searchPlaceholder")
            }
            className="h-8 pl-8 pr-16"
          />
          <div className="pointer-events-none absolute right-2 top-1/2 flex -translate-y-1/2 items-center gap-1">
            {search ? (
              <button
                type="button"
                className="pointer-events-auto rounded p-0.5 text-[var(--muted)] hover:text-[var(--ink)]"
                onClick={() => updateSearch("")}
                aria-label={t("toolbar.clearSearch")}
              >
                <X size={12} />
              </button>
            ) : (
              <span className="kbd">⌘F</span>
            )}
          </div>
        </div>
      </div>

      <div className="no-drag ml-auto flex shrink-0 items-center gap-1">
        {Object.keys(pendingEdits).length > 0 && (
          <Button
            variant="gold"
            size="sm"
            className="h-8"
            onClick={() => setPendingOpen(true)}
            title={t("toolbar.pending")}
          >
            <ClipboardList size={13} />
            <span className="hidden xl:inline">{t("toolbar.pendingShort")}</span>
            <span className="rounded-full bg-black/10 px-1 font-mono text-[9px]">
              {Object.keys(pendingEdits).length}
            </span>
          </Button>
        )}
        <Select
          value={dataMode}
          onChange={(event) => setDataMode(event.target.value as DataMode)}
          className="h-8 w-[118px] text-[10px]"
          title={t("toolbar.modeHint")}
        >
          <option value="full">{t("toolbar.modeFull")}</option>
          <option value="fair_play">{t("toolbar.modeFair")}</option>
          <option value="moneyball">{t("toolbar.modeMoneyball")}</option>
        </Select>
        <Button
          variant={compareUids.length > 0 ? "gold" : "ghost"}
          size="sm"
          className="h-8"
          disabled={compareUids.length < 2}
          onClick={() => setCompareOpen(true)}
          title={t("toolbar.compare")}
        >
          <GitCompareArrows size={13} />
          <span className="hidden xl:inline">{t("toolbar.compareShort")}</span>
          {compareUids.length > 0 && (
            <span className="rounded-full bg-black/10 px-1 font-mono text-[9px]">{compareUids.length}</span>
          )}
        </Button>
        <div className="mx-0.5 h-4 w-px bg-[var(--line)]" />
        <Button
          variant="ghost"
          size="sm"
          className="h-8"
          onClick={() => setAnalysisOpen(true)}
          title={t("toolbar.analysis")}
        >
          <BarChart3 size={13} />
          <span className="hidden xl:inline">{t("toolbar.analysisShort")}</span>
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="h-7 w-7"
          onClick={() => setExportOpen(true)}
          title={t("toolbar.export")}
        >
          <Download size={14} />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="h-7 w-7"
          onClick={() => setSettingsOpen(true)}
          title={t("toolbar.settings")}
        >
          <Settings2 size={14} />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="h-7 w-7"
          onClick={() => setHelpOpen(true)}
          title={t("toolbar.help")}
        >
          <HelpCircle size={14} />
        </Button>
      </div>
    </header>
  );
}
