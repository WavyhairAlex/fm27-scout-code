import { describe, expect, it } from "vitest";
import {
  MOCK_PLAYERS,
  MOCK_SAVE_ID,
  mockAnalysisOverview,
  mockClubDetail,
  mockClubFilterOptions,
  mockQuery,
  mockQueryClubs,
  mockSquadAnalysis,
} from "./mock";

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
