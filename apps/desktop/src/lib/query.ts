import { keepPreviousData, useQuery } from "@tanstack/react-query";
import {
  appInfo,
  getAnalysisOverview,
  getClub,
  getClubFilterOptions,
  getFilterOptions,
  getGrowthHistory,
  getPlayer,
  getPlayerRoles,
  getSquadAnalysis,
  listSaves,
  queryClubs,
  queryPlayers,
  queryStatsMoneyball,
  queryValueFinder,
} from "./api";
import type {
  ClubQuery,
  PlayerQuery,
  SortRule,
  StatsMoneyballRequest,
  ValueFinderRequest,
} from "./types";
import { useAppStore, type ClubFilters, type Filters } from "@/store/app";

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
    generated: filters.generated,
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

export function buildClubQuery(
  saveId: number,
  filters: ClubFilters,
  sort: SortRule[],
  page: number,
  pageSize: number,
): ClubQuery {
  const primary = sort[0];
  return {
    save_id: saveId,
    search: filters.search || null,
    nation_ids: filters.nationIds,
    genders: filters.genders,
    reputation_min: filters.reputationMin,
    reputation_max: filters.reputationMax,
    has_finances: filters.hasFinances,
    balance_min: filters.balanceMin,
    balance_max: filters.balanceMax,
    transfer_budget_min: filters.transferBudgetMin,
    transfer_budget_max: filters.transferBudgetMax,
    wage_budget_min: filters.wageBudgetMin,
    wage_budget_max: filters.wageBudgetMax,
    corporate_facilities_min: filters.corporateFacilitiesMin,
    corporate_facilities_max: filters.corporateFacilitiesMax,
    training_facilities_min: filters.trainingFacilitiesMin,
    training_facilities_max: filters.trainingFacilitiesMax,
    youth_facilities_min: filters.youthFacilitiesMin,
    youth_facilities_max: filters.youthFacilitiesMax,
    junior_coaching_min: filters.juniorCoachingMin,
    junior_coaching_max: filters.juniorCoachingMax,
    youth_recruitment_min: filters.youthRecruitmentMin,
    youth_recruitment_max: filters.youthRecruitmentMax,
    player_count_min: filters.playerCountMin,
    has_stadium: filters.hasStadium,
    sort,
    sort_by: primary?.by ?? "reputation",
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

export function useAppInfo(enabled = true) {
  return useQuery({
    queryKey: ["app-info"],
    queryFn: appInfo,
    enabled,
    staleTime: Infinity,
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

export function useValueFinder(request: ValueFinderRequest | null) {
  return useQuery({
    queryKey: ["value-finder", request],
    queryFn: () => queryValueFinder(request!),
    enabled: request != null,
    placeholderData: keepPreviousData,
    staleTime: 30_000,
  });
}

export function useStatsMoneyball(request: StatsMoneyballRequest | null) {
  return useQuery({
    queryKey: ["stats-moneyball", request],
    queryFn: () => queryStatsMoneyball(request!),
    enabled: request != null,
    placeholderData: keepPreviousData,
    staleTime: 30_000,
  });
}

export function useClubPage() {
  const saveId = useAppStore((state) => state.saveId);
  const filters = useAppStore((state) => state.clubFilters);
  const sort = useAppStore((state) => state.clubSort);
  const page = useAppStore((state) => state.clubPage);
  const pageSize = useAppStore((state) => state.clubPageSize);
  const query = saveId ? buildClubQuery(saveId, filters, sort, page, pageSize) : null;
  return useQuery({
    queryKey: ["clubs", query],
    queryFn: () => queryClubs(query!),
    enabled: query != null,
    placeholderData: keepPreviousData,
    staleTime: 15_000,
  });
}

export function useClubDetail(uid: number | null) {
  const saveId = useAppStore((state) => state.saveId);
  return useQuery({
    queryKey: ["club", saveId, uid],
    queryFn: () => getClub(saveId!, uid!),
    enabled: saveId != null && uid != null,
    staleTime: 60_000,
  });
}

export function useClubFilterOptions() {
  const saveId = useAppStore((state) => state.saveId);
  return useQuery({
    queryKey: ["club-filter-options", saveId],
    queryFn: () => getClubFilterOptions(saveId!),
    enabled: saveId != null,
    staleTime: 60_000,
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
  // Filter options do not depend on table sorting; excluding it avoids a full
  // facet recomputation whenever the user changes the sort order.
  const query = saveId
    ? buildPlayerQuery(saveId, filters, [], 0, 1, hiddenEnabled)
    : null;
  return useQuery({
    queryKey: ["filter-options", query],
    queryFn: () => getFilterOptions(saveId!, query),
    enabled: saveId != null,
    placeholderData: keepPreviousData,
    staleTime: 15_000,
  });
}
