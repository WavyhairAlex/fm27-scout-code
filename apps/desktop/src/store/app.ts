import { create } from "zustand";
import {
  isLanguage,
  resolveLocale,
  translate,
  type Language,
} from "@/lib/i18n";
import {
  HIDDEN_FILTERS,
  type FilterOptions,
  type ParseProgress,
  type SaveSummary,
  type SortRule,
} from "@/lib/types";

const SAVED_VIEWS_KEY = "fm27scout.views.v1";
const LANGUAGE_KEY = "fm27scout.language.v1";

export interface AttributeRule {
  id: string;
  key: string;
  min: number | null;
  max: number | null;
}

export interface Filters {
  search: string;
  clubUids: number[];
  nationIds: number[];
  genders: number[];
  availability: string | null;
  listings: string[];
  ageMin: number | null;
  ageMax: number | null;
  caMin: number | null;
  caMax: number | null;
  paMin: number | null;
  paMax: number | null;
  paCaGapMin: number | null;
  paUnknown: boolean | null;
  positions: string[];
  positionMin: number;
  attributes: AttributeRule[];
  valueMin: number | null;
  valueMax: number | null;
  wageMin: number | null;
  wageMax: number | null;
  contractEndBefore: string | null;
  contractEndAfter: string | null;
}

export const EMPTY_FILTERS: Filters = {
  search: "",
  clubUids: [],
  nationIds: [],
  genders: [],
  availability: null,
  listings: [],
  ageMin: null,
  ageMax: null,
  caMin: null,
  caMax: null,
  paMin: null,
  paMax: null,
  paCaGapMin: null,
  paUnknown: null,
  positions: [],
  positionMin: 15,
  attributes: [],
  valueMin: null,
  valueMax: null,
  wageMin: null,
  wageMax: null,
  contractEndBefore: null,
  contractEndAfter: null,
};

export interface SavedView {
  id: string;
  name: string;
  filters: Filters;
  sort?: SortRule[];
}

export type Theme = "system" | "light" | "dark";
export type DisplayMode = "display" | "raw";

export interface Toast {
  id: number;
  title: string;
  description?: string;
  tone: "default" | "success" | "error";
}

interface AppState {
  saveId: number | null;
  savedViews: SavedView[];
  summary: SaveSummary | null;
  filterOptions: FilterOptions | null;
  filters: Filters;
  sort: SortRule[];
  page: number;
  pageSize: number;
  selectedUid: number | null;
  compareUids: number[];
  hiddenEnabled: boolean;
  displayMode: DisplayMode;
  theme: Theme;
  language: Language;
  sidebarOpen: boolean;
  inspectorOpen: boolean;
  settingsOpen: boolean;
  compareOpen: boolean;
  analysisOpen: boolean;
  exportOpen: boolean;
  helpOpen: boolean;
  toasts: Toast[];
  parsing: boolean;
  parseProgress: ParseProgress | null;
  setSession: (saveId: number, summary: SaveSummary | null) => void;
  clearSession: () => void;
  setFilterOptions: (options: FilterOptions | null) => void;
  patchFilters: (patch: Partial<Filters>) => void;
  resetFilters: () => void;
  saveView: (name: string) => void;
  applyView: (id: string) => void;
  deleteView: (id: string) => void;
  togglePosition: (position: string) => void;
  addAttribute: (key: string) => void;
  updateAttribute: (id: string, patch: Partial<AttributeRule>) => void;
  removeAttribute: (id: string) => void;
  setSort: (by: string, additive?: boolean) => void;
  setSortRules: (rules: SortRule[]) => void;
  setPage: (page: number) => void;
  setSelectedUid: (uid: number | null) => void;
  toggleCompare: (uid: number) => void;
  clearCompare: () => void;
  setHiddenEnabled: (enabled: boolean) => void;
  setDisplayMode: (mode: DisplayMode) => void;
  setTheme: (theme: Theme) => void;
  setLanguage: (language: Language) => void;
  setSidebarOpen: (open: boolean) => void;
  setInspectorOpen: (open: boolean) => void;
  setSettingsOpen: (open: boolean) => void;
  setCompareOpen: (open: boolean) => void;
  setAnalysisOpen: (open: boolean) => void;
  setExportOpen: (open: boolean) => void;
  setHelpOpen: (open: boolean) => void;
  setParsing: (parsing: boolean) => void;
  setParseProgress: (progress: ParseProgress | null) => void;
  pushToast: (toast: Omit<Toast, "id">) => void;
  dismissToast: (id: number) => void;
}

let toastId = 0;

export const useAppStore = create<AppState>((set, get) => ({
  saveId: null,
  savedViews: loadSavedViews(),
  summary: null,
  filterOptions: null,
  filters: EMPTY_FILTERS,
  sort: [{ by: "ca", dir: "desc" }],
  page: 0,
  pageSize: 150,
  selectedUid: null,
  compareUids: [],
  hiddenEnabled: false,
  displayMode: "display",
  theme: "system",
  language: loadLanguage(),
  sidebarOpen: true,
  inspectorOpen: true,
  settingsOpen: false,
  compareOpen: false,
  analysisOpen: false,
  exportOpen: false,
  helpOpen: false,
  toasts: [],
  parsing: false,
  parseProgress: null,
  setSession: (saveId, summary) =>
    set({
      saveId,
      summary,
      filters: EMPTY_FILTERS,
      sort: [{ by: "ca", dir: "desc" }],
      page: 0,
      selectedUid: null,
      compareUids: [],
    }),
  clearSession: () =>
    set({
      saveId: null,
      summary: null,
      filterOptions: null,
      selectedUid: null,
      compareUids: [],
      settingsOpen: false,
      compareOpen: false,
      analysisOpen: false,
      exportOpen: false,
    }),
  setFilterOptions: (filterOptions) => set({ filterOptions }),
  patchFilters: (patch) =>
    set((state) => ({ filters: { ...state.filters, ...patch }, page: 0 })),
  resetFilters: () => set({ filters: EMPTY_FILTERS, page: 0 }),
  saveView: (name) => {
    const trimmed = name.trim();
    if (!trimmed) return;
    const snapshot = JSON.parse(JSON.stringify(get().filters)) as Filters;
    const view: SavedView = {
      id: `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      name: trimmed,
      filters: snapshot,
      sort: get().sort,
    };
    const savedViews = [...get().savedViews, view].slice(-20);
    persistSavedViews(savedViews);
    set({ savedViews });
  },
  applyView: (id) => {
    const view = get().savedViews.find((item) => item.id === id);
    if (!view) return;
    const availability =
      view.filters.availability === "injury_history"
        ? null
        : (view.filters.availability ?? null);
    const enableHidden =
      !get().hiddenEnabled &&
      (view.filters.attributes ?? []).some((rule) => HIDDEN_FILTERS.has(rule.key));
    if (enableHidden) {
      const locale = resolveLocale(get().language);
      get().pushToast({
        title: translate(locale, "toast.hiddenOn.title"),
        description: translate(locale, "toast.hiddenOn.description"),
        tone: "default",
      });
    }
    set({
      filters: {
        ...EMPTY_FILTERS,
        ...view.filters,
        availability,
        attributes: view.filters.attributes ?? [],
        genders: view.filters.genders ?? [],
        listings: view.filters.listings ?? [],
      },
      ...(enableHidden ? { hiddenEnabled: true } : {}),
      sort: view.sort?.length ? view.sort : [{ by: "ca", dir: "desc" }],
      page: 0,
      selectedUid: null,
    });
  },
  deleteView: (id) => {
    const savedViews = get().savedViews.filter((view) => view.id !== id);
    persistSavedViews(savedViews);
    set({ savedViews });
  },
  togglePosition: (position) =>
    set((state) => {
      const positions = state.filters.positions.includes(position)
        ? state.filters.positions.filter((item) => item !== position)
        : [...state.filters.positions, position];
      return { filters: { ...state.filters, positions }, page: 0 };
    }),
  addAttribute: (key) =>
    set((state) => {
      if (state.filters.attributes.some((rule) => rule.key === key)) return state;
      const enableHidden = HIDDEN_FILTERS.has(key) && !state.hiddenEnabled;
      if (enableHidden) {
        const locale = resolveLocale(state.language);
        get().pushToast({
          title: translate(locale, "toast.hiddenOn.title"),
          description: translate(locale, "toast.hiddenOn.description"),
          tone: "default",
        });
      }
      return {
        filters: {
          ...state.filters,
          attributes: [
            ...state.filters.attributes,
            { id: `${key}-${Date.now()}`, key, min: null, max: null },
          ],
        },
        ...(enableHidden ? { hiddenEnabled: true } : {}),
        page: 0,
      };
    }),
  updateAttribute: (id, patch) =>
    set((state) => ({
      filters: {
        ...state.filters,
        attributes: state.filters.attributes.map((rule) =>
          rule.id === id ? { ...rule, ...patch } : rule,
        ),
      },
      page: 0,
    })),
  removeAttribute: (id) =>
    set((state) => ({
      filters: {
        ...state.filters,
        attributes: state.filters.attributes.filter((rule) => rule.id !== id),
      },
      page: 0,
    })),
  setSort: (by, additive = false) =>
    set((state) => {
      if (!additive) {
        const current = state.sort[0];
        const dir: "asc" | "desc" =
          state.sort.length === 1 && current?.by === by && current.dir === "desc"
            ? "asc"
            : "desc";
        const next: SortRule[] = [{ by, dir }];
        return { sort: next, page: 0 };
      }
      const existing = state.sort.find((rule) => rule.by === by);
      if (existing) {
        const next: SortRule[] = state.sort.map((rule) => {
          if (rule.by !== by) return rule;
          const dir: "asc" | "desc" = rule.dir === "desc" ? "asc" : "desc";
          return { ...rule, dir };
        });
        return { sort: next, page: 0 };
      }
      const appended: SortRule = { by, dir: "desc" };
      const next: SortRule[] = [...state.sort, appended].slice(0, 4);
      return { sort: next, page: 0 };
    }),
  setSortRules: (sort) => set({ sort, page: 0 }),
  setPage: (page) => set({ page }),
  setSelectedUid: (selectedUid) =>
    set({ selectedUid, inspectorOpen: selectedUid != null }),
  toggleCompare: (uid) =>
    set((state) => {
      if (state.compareUids.includes(uid)) {
        return { compareUids: state.compareUids.filter((item) => item !== uid) };
      }
      if (state.compareUids.length >= 4) {
        const locale = resolveLocale(state.language);
        get().pushToast({
          title: translate(locale, "toast.compareLimit.title"),
          description: translate(locale, "toast.compareLimit.description"),
          tone: "default",
        });
        return state;
      }
      return { compareUids: [...state.compareUids, uid] };
    }),
  clearCompare: () => set({ compareUids: [] }),
  setHiddenEnabled: (hiddenEnabled) =>
    set((state) => ({
      hiddenEnabled,
      filters: hiddenEnabled
        ? state.filters
        : {
            ...state.filters,
            attributes: state.filters.attributes.filter(
              (rule) => !HIDDEN_FILTERS.has(rule.key),
            ),
          },
      page: hiddenEnabled ? state.page : 0,
    })),
  setDisplayMode: (displayMode) => set({ displayMode }),
  setTheme: (theme) => set({ theme }),
  setLanguage: (language) => {
    persistLanguage(language);
    set({ language });
  },
  setSidebarOpen: (sidebarOpen) => set({ sidebarOpen }),
  setInspectorOpen: (inspectorOpen) => set({ inspectorOpen }),
  setSettingsOpen: (settingsOpen) => set({ settingsOpen }),
  setCompareOpen: (compareOpen) => set({ compareOpen }),
  setAnalysisOpen: (analysisOpen) => set({ analysisOpen }),
  setExportOpen: (exportOpen) => set({ exportOpen }),
  setHelpOpen: (helpOpen) => set({ helpOpen }),
  setParsing: (parsing) => set({ parsing, parseProgress: parsing ? get().parseProgress : null }),
  setParseProgress: (parseProgress) => set({ parseProgress }),
  pushToast: (toast) =>
    set((state) => {
      const id = ++toastId;
      window.setTimeout(() => {
        useAppStore.setState((current) => ({
          toasts: current.toasts.filter((item) => item.id !== id),
        }));
      }, 4200);
      return { toasts: [...state.toasts, { ...toast, id }] };
    }),
  dismissToast: (id) =>
    set((state) => ({ toasts: state.toasts.filter((item) => item.id !== id) })),
}));

function loadLanguage(): Language {
  if (typeof window === "undefined") return "system";
  try {
    const value = window.localStorage.getItem(LANGUAGE_KEY);
    return isLanguage(value) ? value : "system";
  } catch {
    return "system";
  }
}

function persistLanguage(language: Language) {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(LANGUAGE_KEY, language);
  } catch {
    // Local storage can be unavailable in hardened webviews; the language still
    // applies for the current session.
  }
}

function loadSavedViews(): SavedView[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = window.localStorage.getItem(SAVED_VIEWS_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as SavedView[];
    return Array.isArray(parsed) ? parsed.filter((view) => view?.id && view?.name && view?.filters) : [];
  } catch {
    return [];
  }
}

function persistSavedViews(views: SavedView[]) {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(SAVED_VIEWS_KEY, JSON.stringify(views));
  } catch {
    // Local storage can be unavailable in hardened webviews; views are non-critical.
  }
}
