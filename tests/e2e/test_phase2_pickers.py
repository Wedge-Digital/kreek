"""Tests E2E des widgets roster/inducement/star-player picker sur la phase 2
("Règles") de la création de compétition.

Ces widgets sont fournis par le BC `references` et chargés par la page hôte
(BC `competitions`) via leur route HTTP (`htmx.ajax`) — jamais par inclusion
de template ni par accès direct au repository `references`. Chaque tier
obtient sa propre instance de chaque widget (`instance_id=tier.id`), avec un
état de sélection totalement indépendant.

Prérequis : serveur kreek lancé en dev (BYPASS_AUTH=true).
"""

from playwright.sync_api import Page, expect


def _goto_rules_page(page: Page, url: str) -> None:
    page.goto(url, wait_until="load")
    page.wait_for_selector(".tier-block", timeout=5000)
    # Les 3 pickers du premier tier se chargent en s'enchaînant (await côté
    # JS) — attendre que le plus lent (star players) ait fini de peupler.
    page.wait_for_selector(".tier-block [data-slot='star'] .roster-chip", timeout=5000)


def test_default_tier_loads_all_three_pickers(page: Page, competition_rules_url):
    _goto_rules_page(page, competition_rules_url)

    tier = page.locator(".tier-block").first
    assert tier.locator("[data-slot='roster'] .roster-chip").count() > 0
    assert tier.locator("[data-slot='inducement'] .inducement-chip").count() > 0
    assert tier.locator("[data-slot='star'] .roster-chip").count() > 0


def test_new_tier_has_all_items_selected_by_default(page: Page, competition_rules_url):
    _goto_rules_page(page, competition_rules_url)

    tier = page.locator(".tier-block").first
    rosters = tier.locator("[data-slot='roster'] .roster-chip")
    expect(rosters.first).to_have_class("roster-chip selected")
    # Tous les chips doivent être sélectionnés par défaut (select_all=true).
    total = rosters.count()
    selected = 0
    for i in range(total):
        cls = rosters.nth(i).get_attribute("class")
        if "selected" in cls:
            selected += 1
    assert selected == total


def test_tiers_have_independent_picker_state(page: Page, competition_rules_url):
    _goto_rules_page(page, competition_rules_url)

    page.click("button[onclick='addTier()']")
    page.wait_for_timeout(300)
    page.wait_for_selector(".tier-block:nth-child(2) [data-slot='star'] .roster-chip", timeout=5000)

    tier0_first_roster = page.locator(".tier-block").nth(0).locator("[data-slot='roster'] .roster-chip").first
    tier1_first_roster = page.locator(".tier-block").nth(1).locator("[data-slot='roster'] .roster-chip").first

    # Désélectionner dans le tier 0 ne doit pas affecter le tier 1 — sinon
    # c'est que l'état de sélection est partagé entre les deux instances
    # (régression vers l'ancien Alpine.store global).
    tier0_first_roster.click()
    page.wait_for_timeout(200)

    expect(tier0_first_roster).not_to_have_class("roster-chip selected")
    expect(tier1_first_roster).to_have_class("roster-chip selected")


def test_removing_a_chip_updates_tier_selection_count(page: Page, competition_rules_url):
    _goto_rules_page(page, competition_rules_url)

    tier = page.locator(".tier-block").first
    inducements = tier.locator("[data-slot='inducement'] .inducement-chip")
    first = inducements.first

    expect(first).to_have_class("inducement-chip selected")
    first.click()
    page.wait_for_timeout(200)
    expect(first).not_to_have_class("inducement-chip selected")

    first.click()
    page.wait_for_timeout(200)
    expect(first).to_have_class("inducement-chip selected")


def test_submit_error_banner_appears_near_form_actions(page: Page, competition_rules_url):
    # Deux tiers neufs partagent par défaut tous les mêmes rosters
    # (select_all=true) — la soumission doit donc échouer avec
    # RosterInMultipleTiers, et l'erreur doit être visible sans avoir à
    # remonter en haut de la page (régression : la bannière était autrefois
    # préfixée en tête de .create-card, hors champ de vision après scroll).
    _goto_rules_page(page, competition_rules_url)

    page.click("button[onclick='addTier()']")
    page.wait_for_timeout(300)
    page.wait_for_selector(".tier-block:nth-child(2) [data-slot='star'] .roster-chip", timeout=5000)

    banner = page.locator("#rules-error-banner")
    expect(banner).to_be_hidden()

    page.click("button[onclick='submitRules()']")
    page.wait_for_timeout(500)

    expect(banner).to_be_visible()
    expect(banner).to_contain_text("présent dans")

    banner_box = banner.bounding_box()
    form_actions_box = page.locator(".form-actions").bounding_box()
    assert banner_box is not None and form_actions_box is not None
    # La bannière doit être juste au-dessus des boutons de navigation, pas en
    # tête de page (à plusieurs centaines de pixels au-dessus).
    assert form_actions_box["y"] - banner_box["y"] < 100
