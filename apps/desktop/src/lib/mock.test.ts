import { describe, expect, it } from "vitest";
import {
  MOCK_PLAYERS,
  MOCK_SAVE_ID,
  MOCK_STAFF,
  mockAnalysisOverview,
  mockClubDetail,
  mockClubFilterOptions,
  mockFilterOptions,
  mockManagedClub,
  mockQuery,
  mockQueryClubs,
  mockQueryStaff,
  mockScoutAdvice,
  mockScoutOverview,
  mockSquadAnalysis,
  mockStaffFilterOptions,
  mockStatsMoneyball,
  mockValueFinder,
} from "./mock";
import { UNKNOWN_GENDER_ID } from "./types";

describe("mock player query", () => {
  it("searches name and club", () => {
    const page = mockQuery({
      save_id: MOCK_SAVE_ID,
      search: "Modrić",
      page_size: 10,
    });
    expect(page.total).toBeGreaterThan(0);
    expect(page.rows[0].name).toContain("Modrić");
  });

  it("filters positions and attributes", () => {
    const page = mockQuery({
      save_id: MOCK_SAVE_ID,
      positions: ["mc"],
      position_min: 18,
      attributes: [{ key: "passing", min: 15 }],
      page_size: 500,
    });
    expect(page.rows.length).toBeGreaterThan(0);
    for (const row of page.rows) {
      expect(row.positions.mc).toBeGreaterThanOrEqual(18);
      expect(row.attributes.passing).toBeGreaterThanOrEqual(15);
    }
  });

  it("filters personality attributes", () => {
    const page = mockQuery({
      save_id: MOCK_SAVE_ID,
      attributes: [{ key: "professionalism", min: 20 }],
      page_size: 500,
    });
    expect(page.rows.length).toBeGreaterThan(0);
    for (const row of page.rows) {
      expect(row.personality?.professionalism ?? 0).toBeGreaterThanOrEqual(20);
    }
  });

  it("filters gender and status", () => {
    const women = mockQuery({
      save_id: MOCK_SAVE_ID,
      genders: [1],
      page_size: 500,
    });
    expect(women.total).toBeGreaterThan(0);
    expect(women.rows.every((row) => row.gender === 1)).toBe(true);

    const suspensions = mockQuery({
      save_id: MOCK_SAVE_ID,
      availability: "suspended",
      page_size: 500,
    });
    expect(suspensions.total).toBeGreaterThan(0);
    expect(suspensions.rows.every((row) => row.suspension_count > 0)).toBe(true);
  });

  it("counts players with unknown gender in the All option", () => {
    const options = mockFilterOptions();
    const total = options.genders.reduce((sum, option) => sum + option.count, 0);
    expect(total).toBe(MOCK_PLAYERS.length);

    const unknown = mockQuery({
      save_id: MOCK_SAVE_ID,
      genders: [UNKNOWN_GENDER_ID],
      page_size: 500,
    });
    expect(unknown.total).toBe(1);
    expect(unknown.rows.every((row) => row.gender == null)).toBe(true);
  });

  it("filters generated players", () => {
    const generated = mockQuery({
      save_id: MOCK_SAVE_ID,
      generated: true,
      page_size: 500,
    });
    expect(generated.total).toBeGreaterThan(0);
    expect(generated.rows.every((row) => row.generated)).toBe(true);

    const database = mockQuery({
      save_id: MOCK_SAVE_ID,
      generated: false,
      page_size: 500,
    });
    expect(database.total).toBeGreaterThan(0);
    expect(database.rows.every((row) => !row.generated)).toBe(true);
  });

  it("filters transfer and loan listings", () => {
    const sale = mockQuery({
      save_id: MOCK_SAVE_ID,
      listings: ["sale"],
      page_size: 500,
    });
    expect(sale.total).toBeGreaterThan(0);
    expect(sale.rows.every((row) => row.transfer_listed)).toBe(true);

    const loan = mockQuery({
      save_id: MOCK_SAVE_ID,
      listings: ["loan"],
      page_size: 500,
    });
    expect(loan.total).toBeGreaterThan(0);
    expect(loan.rows.every((row) => row.loan_listed)).toBe(true);
  });

  it("applies multi-column sort", () => {
    const page = mockQuery({
      save_id: MOCK_SAVE_ID,
      sort: [
        { by: "ca", dir: "desc" },
        { by: "age", dir: "asc" },
      ],
      page_size: 500,
    });
    for (let index = 1; index < page.rows.length; index += 1) {
      const previous = page.rows[index - 1];
      const current = page.rows[index];
      expect(previous.ca).toBeGreaterThanOrEqual(current.ca);
      if (previous.ca === current.ca) {
        expect(previous.age ?? 999).toBeLessThanOrEqual(current.age ?? 999);
      }
    }
  });

  it("sorts by CA descending and paginates", () => {
    const page = mockQuery({
      save_id: MOCK_SAVE_ID,
      sort_by: "ca",
      sort_dir: "desc",
      page: 0,
      page_size: 5,
    });
    expect(page.rows).toHaveLength(5);
    expect(page.rows[0].ca).toBeGreaterThanOrEqual(page.rows[4].ca);
  });
});

  it("aggregates analysis overview and squad depth", () => {
    const overview = mockAnalysisOverview();
    expect(overview.total_players).toBe(MOCK_PLAYERS.length);
    expect(overview.age_bands.length).toBe(6);
    expect(overview.top_clubs.length).toBeGreaterThan(0);

    const clubUid = MOCK_PLAYERS[0].club_uid;
    expect(clubUid).not.toBeNull();
    const squad = mockSquadAnalysis(clubUid!);
    expect(squad.player_count).toBeGreaterThan(0);
    expect(squad.players[0].club_uid).toBe(clubUid);

    const managed = mockManagedClub();
    expect(managed.club_uid).toBe(clubUid);
    const scoped = mockAnalysisOverview(managed.club_uid);
    expect(scoped.total_players).toBe(
      MOCK_PLAYERS.filter((player) => player.club_uid === managed.club_uid).length,
    );
    expect(scoped.total_players).toBeLessThan(overview.total_players);
    expect(scoped.total_clubs).toBe(overview.total_clubs);
    expect(scoped.top_clubs.some((club) => club.club_uid === managed.club_uid)).toBe(true);
  });

describe("mock scouting", () => {
  it("builds squad depth and recommendations for the managed club", () => {
    const managed = mockManagedClub();
    const overview = mockScoutOverview(managed.club_uid);
    expect(overview.lines).toHaveLength(10);
    expect(overview.positions.length).toBeGreaterThan(0);
    expect(overview.roles.length).toBeGreaterThan(0);
    expect(overview.squad_size).toBeGreaterThan(0);

    const position =
      overview.lines.find((line) => line.status === "missing" || line.status === "thin")
        ?.positions[0] ?? "stc";
    const advice = mockScoutAdvice({
      save_id: MOCK_SAVE_ID,
      club_uid: managed.club_uid,
      position,
      purpose: "any",
      how_many: 10,
    });
    expect(advice.suggestions.length).toBeLessThanOrEqual(10);
    for (const suggestion of advice.suggestions) {
      expect(suggestion.fit).toBeGreaterThanOrEqual(0);
      expect(suggestion.fit).toBeLessThanOrEqual(100);
      expect(suggestion.probability).toBeGreaterThanOrEqual(0);
      expect(suggestion.probability).toBeLessThanOrEqual(100);
      expect(suggestion.player.club_uid).not.toBe(managed.club_uid);
    }
  });
});

describe("mock club query", () => {
  it("searches clubs by name", () => {
    const page = mockQueryClubs({
      save_id: MOCK_SAVE_ID,
      search: "Barcelona",
      page_size: 10,
    });
    expect(page.total).toBeGreaterThan(0);
    expect(page.rows[0].name).toContain("Barcelona");
  });

  it("filters clubs with finances and sorts by balance", () => {
    const page = mockQueryClubs({
      save_id: MOCK_SAVE_ID,
      has_finances: true,
      sort_by: "balance",
      sort_dir: "desc",
      page_size: 20,
    });
    expect(page.rows.every((row) => row.finance_month != null)).toBe(true);
    for (let index = 1; index < page.rows.length; index += 1) {
      expect(page.rows[index - 1].balance ?? 0).toBeGreaterThanOrEqual(
        page.rows[index].balance ?? 0,
      );
    }
  });

  it("returns a detail with finances and a stadium", () => {
    const detail = mockClubDetail(1708);
    expect(detail).not.toBeNull();
    expect(detail!.finance_months.length).toBeGreaterThan(0);
    expect(detail!.stadiums.length).toBeGreaterThan(0);
    expect(mockClubFilterOptions().clubs_with_finances).toBeGreaterThan(0);
  });
});

  it("applies the PA-CA gap filter", () => {
    const page = mockQuery({
      save_id: MOCK_SAVE_ID,
      pa_ca_gap_min: 10,
      page_size: 500,
    });
    for (const row of page.rows) {
      expect(row.pa).not.toBeNull();
      expect((row.pa ?? 0) - row.ca).toBeGreaterThanOrEqual(10);
    }
  });

describe("mock value finder", () => {
  it("returns candidates with defaults", () => {
    const page = mockValueFinder({
      save_id: MOCK_SAVE_ID,
      ca_min: 120,
      role_min: 12,
      age_max: 30,
      include_unknown_fee: false,
      page: 0,
      page_size: 40,
      sort_by: "score",
      sort_dir: "desc",
    });
    expect(page.total).toBeGreaterThan(0);
    expect(page.rows.length).toBeGreaterThan(0);
  });
});

describe("mock staff query", () => {
  it("searches staff by name, club and job", () => {
    const page = mockQueryStaff({
      save_id: MOCK_SAVE_ID,
      search: "Coach",
      page_size: 200,
    });
    expect(page.total).toBeGreaterThan(0);
    for (const row of page.rows) {
      const haystack = [row.name, row.club_name, row.nation_name, row.job_name]
        .filter(Boolean)
        .join(" ")
        .toLowerCase();
      expect(haystack).toContain("coach");
    }
  });

  it("filters role suitability and profile attributes", () => {
    const page = mockQueryStaff({
      save_id: MOCK_SAVE_ID,
      roles: [{ key: "coach", min: 15 }],
      attributes: [{ key: "motivating", min: 12 }],
      has_contract: true,
      page_size: 200,
    });
    expect(page.total).toBeGreaterThan(0);
    for (const row of page.rows) {
      expect(row.roles?.[2] ?? 0).toBeGreaterThanOrEqual(15);
      expect(row.attributes?.motivating ?? 0).toBeGreaterThanOrEqual(12);
      expect(row.has_contract).toBe(true);
    }
  });

  it("sorts and pages staff rows", () => {
    const first = mockQueryStaff({
      save_id: MOCK_SAVE_ID,
      sort: [{ by: "role", dir: "desc" }],
      page: 0,
      page_size: 10,
    });
    const second = mockQueryStaff({
      save_id: MOCK_SAVE_ID,
      sort: [{ by: "role", dir: "desc" }],
      page: 1,
      page_size: 10,
    });
    expect(first.total).toBe(second.total);
    expect(first.rows[0].best_role_score ?? 0).toBeGreaterThanOrEqual(
      first.rows[1].best_role_score ?? 0,
    );
    expect(first.rows.some((row) => second.rows.some((next) => next.uid === row.uid))).toBe(false);
  });

  it("provides staff facets and value ranges", () => {
    const options = mockStaffFilterOptions();
    expect(options.clubs.length).toBeGreaterThan(0);
    expect(options.jobs.length).toBeGreaterThan(0);
    expect(options.with_contract + options.without_contract).toBe(MOCK_STAFF.length);
    expect(options.with_attributes).toBe(MOCK_STAFF.length);
    expect(options.with_roles).toBe(MOCK_STAFF.length);
  });
});

describe("mock stats moneyball", () => {
  it("scores the core model and respects constraints", () => {
    const page = mockStatsMoneyball({
      save_id: MOCK_SAVE_ID,
      model: "core",
      ca_min: 120,
      age_max: 30,
      min_minutes: 900,
      page_size: 40,
    });
    expect(page.total).toBeGreaterThan(0);
    for (const row of page.rows) {
      expect(row.ca).toBeGreaterThanOrEqual(120);
      expect(row.age ?? 0).toBeLessThanOrEqual(30);
      expect(row.minutes).toBeGreaterThanOrEqual(900);
      expect(row.score).not.toBeNull();
    }
  });

  it("only returns advanced rows for the advanced model", () => {
    const page = mockStatsMoneyball({
      save_id: MOCK_SAVE_ID,
      model: "advanced",
      ca_min: 120,
      page_size: 40,
    });
    expect(page.total).toBeGreaterThan(0);
    expect(page.rows.every((row) => row.advanced_available)).toBe(true);
  });
});
