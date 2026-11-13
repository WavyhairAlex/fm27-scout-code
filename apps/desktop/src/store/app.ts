import { create } from "zustand";
import {
  isLanguage,
  resolveLocale,
  translate,
  type Language,
} from "@/lib/i18n";
import { isCurrencyCode, type CurrencyCode } from "@/lib/currency";
import {
  type FilterOptions,
  type ParseProgress,
  type PlayerEditRequest,
  type SaveSummary,
  type SortRule,
} from "@/lib/types";

const SAVED_VIEWS_KEY = "fm27scout.views.v1";
const LANGUAGE_KEY = "fm27scout.language.v1";
const CURRENCY_KEY = "fm27scout.currency.v1";
const DATA_MODE_KEY = "fm27scout.dataMode.v1";

export interface AttributeRule {
  id: string;
  key: string;
  min: number | null;
  max: number | null;
}

export interface StaffRoleRule {
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
  generated: boolean | null;
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

export interface ClubFilters {
  search: string;
  nationIds: number[];
  genders: number[];
  reputationMin: number | null;
  reputationMax: number | null;
  hasFinances: boolean | null;
  balanceMin: number | null;
  balanceMax: number | null;
  transferBudgetMin: number | null;
  transferBudgetMax: number | null;
  wageBudgetMin: number | null;
  wageBudgetMax: number | null;
  corporateFacilitiesMin: number | null;
  corporateFacilitiesMax: number | null;
  trainingFacilitiesMin: number | null;
  trainingFacilitiesMax: number | null;
  youthFacilitiesMin: number | null;
  youthFacilitiesMax: number | null;
  juniorCoachingMin: number | null;
  juniorCoachingMax: number | null;
  youthRecruitmentMin: number | null;
  youthRecruitmentMax: number | null;
  playerCountMin: number | null;
  hasStadium: boolean | null;
}

export const EMPTY_CLUB_FILTERS: ClubFilters = {
  search: "",
  nationIds: [],
  genders: [],
  reputationMin: null,
  reputationMax: null,
  hasFinances: null,
  balanceMin: null,
  balanceMax: null,
  transferBudgetMin: null,
  transferBudgetMax: null,
  wageBudgetMin: null,
  wageBudgetMax: null,
  corporateFacilitiesMin: null,
  corporateFacilitiesMax: null,
  trainingFacilitiesMin: null,
  trainingFacilitiesMax: null,
  youthFacilitiesMin: null,
  youthFacilitiesMax: null,
  juniorCoachingMin: null,
  juniorCoachingMax: null,
  youthRecruitmentMin: null,
  youthRecruitmentMax: null,
  playerCountMin: null,
  hasStadium: null,
};

export interface StaffFilters {
  search: string;
  clubUids: number[];
  nationIds: number[];
  jobCodes: number[];
  ageMin: number | null;
  ageMax: number | null;
  caMin: number | null;
  caMax: number | null;
  paMin: number | null;
  paMax: number | null;
  wageMin: number | null;
  wageMax: number | null;
  contractEndBefore: string | null;
  contractEndAfter: string | null;
  hasContract: boolean | null;
  attributes: AttributeRule[];
  roles: StaffRoleRule[];
}

export const EMPTY_STAFF_FILTERS: StaffFilters = {
  search: "",
  clubUids: [],
  nationIds: [],
  jobCodes: [],
  ageMin: null,
  ageMax: null,
  caMin: null,
  caMax: null,
  paMin: null,
  paMax: null,
  wageMin: null,
  wageMax: null,
  contractEndBefore: null,
  contractEndAfter: null,
  hasContract: null,
  attributes: [],
  roles: [],
};

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
  generated: null,
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
export type DataMode = "full" | "fair_play" | "moneyball";

export interface Toast {
  id: number;
  title: string;
  description?: string;
  tone: "default" | "success" | "error";
}

export interface PendingChange {
  field: string;
  before: number;
  after: number;
  label?: string;
  before_display?: string | null;
  after_display?: string | null;
}

export interface PendingEdit {
  uid: number;
  name: string;
  changes: PendingChange[];
  request: PlayerEditRequest;
}

export type ViewMode = "players" | "clubs" | "staff";

interface AppState {
  saveId: number | null;
  savedViews: SavedView[];
  summary: SaveSummary | null;
  viewMode: ViewMode;
  filterOptions: FilterOptions | null;
  filters: Filters;
  sort: SortRule[];
  page: number;
  pageSize: number;
  selectedUid: number | null;
  clubFilters: ClubFilters;
  clubSort: SortRule[];
  clubPage: number;
  clubPageSize: number;
  selectedClubUid: number | null;
  clubEditOpen: boolean;
  staffFilters: StaffFilters;
  staffSort: SortRule[];
  staffPage: number;
  staffPageSize: number;
  selectedStaffUid: number | null;
  compareUids: number[];
  dataMode: DataMode;
  displayMode: DisplayMode;
  theme: Theme;
  language: Language;
  currency: CurrencyCode;
  sidebarOpen: boolean;
  inspectorOpen: boolean;
  settingsOpen: boolean;
  compareOpen: boolean;
  analysisOpen: boolean;
  exportOpen: boolean;
  editOpen: boolean;
  pendingOpen: boolean;
  pendingEdits: Record<number, PendingEdit>;
  helpOpen: boolean;
  toasts: Toast[];
  parsing: boolean;
  parseProgress: ParseProgress | null;
  setSession: (saveId: number, summary: SaveSummary | null) => void;
  clearSession: () => void;
  setViewMode: (mode: ViewMode) => void;
  setFilterOptions: (options: FilterOptions | null) => void;
  patchFilters: (patch: Partial<Filters>) => void;
  resetFilters: () => void;
  patchClubFilters: (patch: Partial<ClubFilters>) => void;
  resetClubFilters: () => void;
  setClubSort: (by: string, additive?: boolean) => void;
  setClubPage: (page: number) => void;
  setSelectedClubUid: (uid: number | null) => void;
  setClubEditOpen: (open: boolean) => void;
  patchStaffFilters: (patch: Partial<StaffFilters>) => void;
  resetStaffFilters: () => void;
  setStaffSort: (by: string, additive?: boolean) => void;
  setStaffPage: (page: number) => void;
  setSelectedStaffUid: (uid: number | null) => void;
  addStaffAttribute: (key: string) => void;
  updateStaffAttribute: (id: string, patch: Partial<AttributeRule>) => void;
  removeStaffAttribute: (id: string) => void;
  addStaffRole: (key: string) => void;
  updateStaffRole: (id: string, patch: Partial<StaffRoleRule>) => void;
  removeStaffRole: (id: string) => void;
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
  setDataMode: (mode: DataMode) => void;
  setDisplayMode: (mode: DisplayMode) => void;
  setTheme: (theme: Theme) => void;
  setLanguage: (language: Language) => void;
  setCurrency: (currency: CurrencyCode) => void;
  setSidebarOpen: (open: boolean) => void;
  setInspectorOpen: (open: boolean) => void;
  setSettingsOpen: (open: boolean) => void;
  setCompareOpen: (open: boolean) => void;
  setAnalysisOpen: (open: boolean) => void;
  setExportOpen: (open: boolean) => void;
  setEditOpen: (open: boolean) => void;
  setPendingOpen: (open: boolean) => void;
  stageEdit: (edit: PendingEdit) => void;
  removePendingEdit: (uid: number) => void;
  clearPendingEdits: () => void;
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
  viewMode: "players",
  filterOptions: null,
  filters: EMPTY_FILTERS,
  sort: loadDataMode() === "full" ? [{ by: "ca", dir: "desc" }] : [{ by: "value", dir: "desc" }],
  page: 0,
  pageSize: 150,
  selectedUid: null,
  clubFilters: EMPTY_CLUB_FILTERS,
  clubSort: [{ by: "reputation", dir: "desc" }],
  clubPage: 0,
  clubPageSize: 100,
  selectedClubUid: null,
  clubEditOpen: false,
  staffFilters: EMPTY_STAFF_FILTERS,
  staffSort: [{ by: "ca", dir: "desc" }],
  staffPage: 0,
  staffPageSize: 100,
  selectedStaffUid: null,
  compareUids: [],
  dataMode: loadDataMode(),
  displayMode: "display",
  theme: "system",
  language: loadLanguage(),
  currency: loadCurrency(),
  sidebarOpen: true,
  inspectorOpen: true,
  settingsOpen: false,
  compareOpen: false,
  analysisOpen: false,
  exportOpen: false,
  editOpen: false,
  pendingOpen: false,
  pendingEdits: {},
  helpOpen: false,
  toasts: [],
  parsing: false,
  parseProgress: null,
  setSession: (saveId, summary) =>
    set({
      saveId,
      summary,
      filters: EMPTY_FILTERS,
      sort:
        get().dataMode === "full"
          ? [{ by: "ca", dir: "desc" }]
          : [{ by: "value", dir: "desc" }],
      page: 0,
      selectedUid: null,
      clubFilters: EMPTY_CLUB_FILTERS,
      clubSort: [{ by: "reputation", dir: "desc" }],
      clubPage: 0,
      selectedClubUid: null,
      clubEditOpen: false,
      staffFilters: EMPTY_STAFF_FILTERS,
      staffSort: [{ by: "ca", dir: "desc" }],
      staffPage: 0,
      selectedStaffUid: null,
      compareUids: [],
      editOpen: false,
      pendingOpen: false,
      pendingEdits: {},
    }),
  clearSession: () =>
    set({
      saveId: null,
      summary: null,
      filterOptions: null,
      selectedUid: null,
      selectedClubUid: null,
      clubEditOpen: false,
      staffFilters: EMPTY_STAFF_FILTERS,
      staffPage: 0,
      selectedStaffUid: null,
      compareUids: [],
      settingsOpen: false,
      compareOpen: false,
      analysisOpen: false,
      exportOpen: false,
      editOpen: false,
      pendingOpen: false,
      pendingEdits: {},
    }),
  setViewMode: (viewMode) => set({ viewMode }),
  setFilterOptions: (filterOptions) => set({ filterOptions }),
  patchFilters: (patch) =>
    set((state) => ({ filters: { ...state.filters, ...patch }, page: 0 })),
  resetFilters: () => set({ filters: EMPTY_FILTERS, page: 0 }),
  patchClubFilters: (patch) =>
    set((state) => ({ clubFilters: { ...state.clubFilters, ...patch }, clubPage: 0 })),
  resetClubFilters: () => set({ clubFilters: EMPTY_CLUB_FILTERS, clubPage: 0 }),
  setClubSort: (by, additive = false) =>
    set((state) => {
      if (!additive) {
        const current = state.clubSort[0];
        const dir: "asc" | "desc" =
          state.clubSort.length === 1 && current?.by === by && current.dir === "desc"
            ? "asc"
            : "desc";
        return { clubSort: [{ by, dir }], clubPage: 0 };
      }
      const existing = state.clubSort.find((rule) => rule.by === by);
      if (existing) {
        return {
          clubSort: state.clubSort.map((rule) =>
            rule.by === by ? { ...rule, dir: rule.dir === "desc" ? "asc" : "desc" } : rule,
          ),
          clubPage: 0,
        };
      }
      return {
        clubSort: [...state.clubSort, { by, dir: "desc" as const }].slice(0, 4),
        clubPage: 0,
      };
    }),
  setClubPage: (clubPage) => set({ clubPage }),
  setSelectedClubUid: (selectedClubUid) =>
    set({ selectedClubUid, inspectorOpen: selectedClubUid != null }),
  setClubEditOpen: (clubEditOpen) => set({ clubEditOpen }),
  patchStaffFilters: (patch) =>
    set((state) => ({ staffFilters: { ...state.staffFilters, ...patch }, staffPage: 0 })),
  resetStaffFilters: () => set({ staffFilters: EMPTY_STAFF_FILTERS, staffPage: 0 }),
  setStaffSort: (by, additive = false) =>
    set((state) => {
      if (!additive) {
        const current = state.staffSort[0];
        const dir: "asc" | "desc" =
          state.staffSort.length === 1 && current?.by === by && current.dir === "desc"
            ? "asc"
            : "desc";
        return { staffSort: [{ by, dir }], staffPage: 0 };
      }
      const existing = state.staffSort.find((rule) => rule.by === by);
      if (existing) {
        return {
          staffSort: state.staffSort.map((rule) =>
            rule.by === by ? { ...rule, dir: rule.dir === "desc" ? "asc" : "desc" } : rule,
          ),
          staffPage: 0,
        };
      }
      return {
        staffSort: [...state.staffSort, { by, dir: "desc" as const }].slice(0, 4),
        staffPage: 0,
      };
    }),
  setStaffPage: (staffPage) => set({ staffPage }),
  setSelectedStaffUid: (selectedStaffUid) =>
    set({ selectedStaffUid, inspectorOpen: selectedStaffUid != null }),
  addStaffAttribute: (key) =>
    set((state) => {
      if (state.staffFilters.attributes.some((rule) => rule.key === key)) return state;
      return {
        staffFilters: {
          ...state.staffFilters,
          attributes: [
            ...state.staffFilters.attributes,
            { id: `${key}-${Date.now()}`, key, min: null, max: null },
          ],
        },
        staffPage: 0,
      };
    }),
  updateStaffAttribute: (id, patch) =>
    set((state) => ({
      staffFilters: {
        ...state.staffFilters,
        attributes: state.staffFilters.attributes.map((rule) =>
          rule.id === id ? { ...rule, ...patch } : rule,
        ),
      },
      staffPage: 0,
    })),
  removeStaffAttribute: (id) =>
    set((state) => ({
      staffFilters: {
        ...state.staffFilters,
        attributes: state.staffFilters.attributes.filter((rule) => rule.id !== id),
      },
      staffPage: 0,
    })),
  addStaffRole: (key) =>
    set((state) => {
      if (state.staffFilters.roles.some((rule) => rule.key === key)) return state;
      return {
        staffFilters: {
          ...state.staffFilters,
          roles: [
            ...state.staffFilters.roles,
            { id: `${key}-${Date.now()}`, key, min: 10, max: null },
          ],
        },
        staffPage: 0,
      };
    }),
  updateStaffRole: (id, patch) =>
    set((state) => ({
      staffFilters: {
        ...state.staffFilters,
        roles: state.staffFilters.roles.map((rule) =>
          rule.id === id ? { ...rule, ...patch } : rule,
        ),
      },
      staffPage: 0,
    })),
  removeStaffRole: (id) =>
    set((state) => ({
      staffFilters: {
        ...state.staffFilters,
        roles: state.staffFilters.roles.filter((rule) => rule.id !== id),
      },
      staffPage: 0,
    })),
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
    const dataMode = get().dataMode;
    const visibleAttributes =
      dataMode === "moneyball" ? [] : (view.filters.attributes ?? []);
    const visibleSort =
      dataMode === "full"
        ? view.sort
        : view.sort?.filter(
            (rule) => rule.by !== "ca" && rule.by !== "pa" && rule.by !== "best_role_score",
          );
    set({
      filters: {
        ...EMPTY_FILTERS,
        ...view.filters,
        availability,
        attributes: visibleAttributes,
        genders: view.filters.genders ?? [],
        listings: view.filters.listings ?? [],
        caMin: dataMode === "full" ? view.filters.caMin : null,
        caMax: dataMode === "full" ? view.filters.caMax : null,
        paMin: dataMode === "full" ? view.filters.paMin : null,
        paMax: dataMode === "full" ? view.filters.paMax : null,
        paCaGapMin: dataMode === "full" ? view.filters.paCaGapMin : null,
      },
      sort: visibleSort?.length
        ? visibleSort
        : dataMode === "full"
          ? [{ by: "ca", dir: "desc" }]
          : [{ by: "value", dir: "desc" }],
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
      return {
        filters: {
          ...state.filters,
          attributes: [
            ...state.filters.attributes,
            { id: `${key}-${Date.now()}`, key, min: null, max: null },
          ],
        },
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
  setDataMode: (dataMode) => {
    persistDataMode(dataMode);
    set((state) => {
      const full = dataMode === "full";
      const visibleAttributes =
        dataMode === "moneyball" ? [] : state.filters.attributes;
      const sortUsesHiddenAbility = state.sort.some(
        (rule) => rule.by === "ca" || rule.by === "pa" || rule.by === "best_role_score",
      );
      return {
        dataMode,
        filters: {
          ...state.filters,
          // CA/PA and the PA-CA gap are not visible outside Full mode.
          caMin: full ? state.filters.caMin : null,
          caMax: full ? state.filters.caMax : null,
          paMin: full ? state.filters.paMin : null,
          paMax: full ? state.filters.paMax : null,
          paCaGapMin: full ? state.filters.paCaGapMin : null,
          attributes: visibleAttributes,
        },
        sort: sortUsesHiddenAbility ? [{ by: "value", dir: "desc" }] : state.sort,
        page: 0,
      };
    });
  },
  setDisplayMode: (displayMode) => set({ displayMode }),
  setTheme: (theme) => set({ theme }),
  setLanguage: (language) => {
    persistLanguage(language);
    set({ language });
  },
  setCurrency: (currency) => {
    persistCurrency(currency);
    set({ currency });
  },
  setSidebarOpen: (sidebarOpen) => set({ sidebarOpen }),
  setInspectorOpen: (inspectorOpen) => set({ inspectorOpen }),
  setSettingsOpen: (settingsOpen) => set({ settingsOpen }),
  setCompareOpen: (compareOpen) => set({ compareOpen }),
  setAnalysisOpen: (analysisOpen) => set({ analysisOpen }),
  setExportOpen: (exportOpen) => set({ exportOpen }),
  setEditOpen: (editOpen) => set({ editOpen }),
  setPendingOpen: (pendingOpen) => set({ pendingOpen }),
  stageEdit: (edit) =>
    set((state) => ({
      pendingEdits: { ...state.pendingEdits, [edit.uid]: edit },
    })),
  removePendingEdit: (uid) =>
    set((state) => {
      const pendingEdits = { ...state.pendingEdits };
      delete pendingEdits[uid];
      return { pendingEdits };
    }),
  clearPendingEdits: () => set({ pendingEdits: {} }),
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

function loadDataMode(): DataMode {
  if (typeof window === "undefined") return "full";
  try {
    const value = window.localStorage.getItem(DATA_MODE_KEY);
    return value === "fair_play" || value === "moneyball" || value === "full"
      ? value
      : "full";
  } catch {
    return "full";
  }
}

function persistDataMode(mode: DataMode) {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(DATA_MODE_KEY, mode);
  } catch {
    // Local storage can be unavailable in hardened webviews.
  }
}

function loadCurrency(): CurrencyCode {
  if (typeof window === "undefined") return "save";
  try {
    const value = window.localStorage.getItem(CURRENCY_KEY);
    return isCurrencyCode(value) ? value : "save";
  } catch {
    return "save";
  }
}

function persistCurrency(currency: CurrencyCode) {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(CURRENCY_KEY, currency);
  } catch {
    // Local storage can be unavailable in hardened webviews; the display
    // currency still applies for the current session.
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
