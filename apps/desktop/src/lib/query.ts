import { keepPreviousData, useQuery } from "@tanstack/react-query";
import {
  getAnalysisOverview,
  getFilterOptions,
  getGrowthHistory,
  getPlayer,
  getPlayerRoles,
  getSquadAnalysis,
  listSaves,
  queryPlayers,
} from "./api";
import type { PlayerQuery, SortRule } from "./types";
import { useAppStore, type Filters } from "@/store/app";

export function buildPlayerQuery(
  saveId: number,
  filters: Filters,
  sort: SortRule[],
  page: number,
  pageSize: number,
  includeHidden: boolean,
): PlayerQuery {
  const primary = sort[0];
  return {
    save_id: saveId,
    search: filters.search || null,
    uids: [],
    club_uids: filters.clubUids,
    nation_ids: filters.nationIds,
    genders: filters.genders,
    availability: filters.availability,
    listings: filters.listings,
    age_min: filters.ageMin,
    age_max: filters.ageMax,
    ca_min: filters.caMin,
    ca_max: filters.caMax,
    pa_min: filters.paMin,
    pa_max: filters.paMax,
    pa_ca_gap_min: filters.paCaGapMin,
    pa_unknown: filters.paUnknown,
    positions: filters.positions,
    position_min: filters.positionMin,
    attributes: filters.attributes.map((rule) => ({
      key: rule.key,
      min: rule.min,
      max: rule.max,
    })),
    value_min: filters.valueMin,
    value_max: filters.valueMax,
    wage_min: filters.wageMin,
    wage_max: filters.wageMax,
    contract_end_before: filters.contractEndBefore,
    contract_end_after: filters.contractEndAfter,
    include_hidden: includeHidden,
    sort,
    sort_by: primary?.by ?? "ca",
    sort_dir: primary?.dir ?? "desc",
    page,
    page_size: pageSize,
  };
}

export function useSaves() {
  return useQuery({
    queryKey: ["saves"],
    queryFn: listSaves,
    staleTime: 10_000,
  });
}

export function usePlayerPage() {
  const saveId = useAppStore((state) => state.saveId);
  const filters = useAppStore((state) => state.filters);
  const sort = useAppStore((state) => state.sort);
  const page = useAppStore((state) => state.page);
  const pageSize = useAppStore((state) => state.pageSize);
  const hiddenEnabled = useAppStore((state) => state.hiddenEnabled);
  const query = saveId
    ? buildPlayerQuery(saveId, filters, sort, page, pageSize, hiddenEnabled)
    : null;
  return useQuery({
    queryKey: ["players", query],
    queryFn: () => queryPlayers(query!),
    enabled: query != null,
    placeholderData: keepPreviousData,
    staleTime: 15_000,
  });
}

export function usePlayerDetail(uid: number | null) {
  const saveId = useAppStore((state) => state.saveId);
  const hiddenEnabled = useAppStore((state) => state.hiddenEnabled);
  return useQuery({
    queryKey: ["player", saveId, uid, hiddenEnabled],
    queryFn: () => getPlayer(saveId!, uid!, hiddenEnabled),
    enabled: saveId != null && uid != null,
    staleTime: 60_000,
  });
}

export function usePlayerEditDetail(uid: number | null) {
  const saveId = useAppStore((state) => state.saveId);
  return useQuery({
    queryKey: ["player-edit", saveId, uid],
    queryFn: () => getPlayer(saveId!, uid!, true),
    enabled: saveId != null && uid != null,
    staleTime: 60_000,
  });
}

export function usePlayerRoles(uid: number | null) {
  const saveId = useAppStore((state) => state.saveId);
  return useQuery({
    queryKey: ["player-roles", saveId, uid],
    queryFn: () => getPlayerRoles(saveId!, uid!),
    enabled: saveId != null && uid != null,
    staleTime: 60_000,
  });
}

export function useGrowthHistory(uid: number | null) {
  const saveId = useAppStore((state) => state.saveId);
  return useQuery({
    queryKey: ["growth", saveId, uid],
    queryFn: () => getGrowthHistory(saveId!, uid!),
    enabled: saveId != null && uid != null,
    staleTime: 60_000,
  });
}

export function useAnalysisOverview(enabled = true) {
  const saveId = useAppStore((state) => state.saveId);
  return useQuery({
    queryKey: ["analysis-overview", saveId],
    queryFn: () => getAnalysisOverview(saveId!),
    enabled: saveId != null && enabled,
    staleTime: 60_000,
  });
}

export function useSquadAnalysis(clubUid: number | null) {
  const saveId = useAppStore((state) => state.saveId);
  return useQuery({
    queryKey: ["squad-analysis", saveId, clubUid],
    queryFn: () => getSquadAnalysis(saveId!, clubUid!),
    enabled: saveId != null && clubUid != null,
    staleTime: 60_000,
  });
}

export function useFilterOptions() {
  const saveId = useAppStore((state) => state.saveId);
  const filters = useAppStore((state) => state.filters);
  const sort = useAppStore((state) => state.sort);
  const hiddenEnabled = useAppStore((state) => state.hiddenEnabled);
  const query = saveId
    ? buildPlayerQuery(saveId, filters, sort, 0, 1, hiddenEnabled)
    : null;
  return useQuery({
    queryKey: ["filter-options", query],
    queryFn: () => getFilterOptions(saveId!, query),
    enabled: saveId != null,
    placeholderData: keepPreviousData,
    staleTime: 15_000,
  });
}
