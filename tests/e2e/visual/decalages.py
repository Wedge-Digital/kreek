"""Mesure le saut de mise en page causé par les zones remplies en différé.

    uv run python visual/decalages.py

# Pourquoi pas `layout-shift`

Le navigateur sait mesurer ça — c'est le CLS — et il rend **zéro** sur cette
application. Vérifié, et l'observateur fonctionne : sur une page minimale, il
détecte un décalage provoqué exprès.

La raison tient au shell :

    .app-layout { height: 100vh; overflow: hidden; }
    .main-area  { height: 100vh; overflow-y: auto; }

Le contenu défile **à l'intérieur** de `.main-area`. Un décalage qui s'y produit
ne déplace rien dans la fenêtre, donc le navigateur ne le compte pas. Le CLS de
l'application est réellement nul, et le contenu saute quand même sous les yeux —
les deux sont vrais.

# Ce qu'on mesure à la place

Deux chargements de la même page :

1. les requêtes HTMX **bloquées** — c'est l'état du premier rendu, zones vides ;
2. normal — c'est l'état final.

On compare la position verticale des **repères de la page**, ceux qui existent
dans les deux états parce qu'ils viennent du template et non des fragments. Leur
déplacement est exactement ce que l'utilisateur voit bouger.

C'est le protocole de diagnostic de la carte 343, sans son piège : elle
proposait de vider les conteneurs puis de restaurer leur `innerHTML`, ce qui
détruit les liaisons HTMX. Bloquer les requêtes ne touche rien.
"""

import sys
from pathlib import Path

from playwright.sync_api import sync_playwright

from urls import collecter

# Les repères : tout élément du shell et du template de page, identifié par une
# clé stable qui ne dépend pas du nombre de frères — donc pas un chemin d'arbre,
# que l'arrivée d'un fragment décalerait.
REPERES = """
() => {
  const out = {};
  const cle = el => {
    const parts = [];
    let e = el, garde = 0;
    while (e && e !== document.body && garde++ < 6) {
      const c = e.id ? '#' + e.id : (e.className ? '.' + String(e.className).trim().split(/\\s+/)[0] : e.tagName.toLowerCase());
      parts.unshift(c);
      e = e.parentElement;
    }
    return parts.join('>');
  };
  for (const el of document.querySelectorAll('.app-layout *')) {
    if (!el.className && !el.id) continue;
    const r = el.getBoundingClientRect();
    if (r.height === 0 && r.width === 0) continue;
    const k = cle(el);
    if (!(k in out)) out[k] = Math.round(r.top * 10) / 10;
  }
  return out;
}
"""


def geometrie(nav, url: str, largeur: int, bloquer: bool) -> dict:
    page = nav.new_page(viewport={"width": largeur, "height": 900})
    if bloquer:
        # Les fragments HTMX portent l'en-tête `HX-Request`. Les couper laisse
        # la page dans l'état exact de son premier rendu.
        page.route("**/*", lambda r: r.abort()
                   if r.request.headers.get("hx-request") else r.continue_())
    try:
        page.goto(url, wait_until="load", timeout=25000)
        page.wait_for_timeout(3500)
        return page.evaluate(REPERES)
    finally:
        page.close()


def main() -> int:
    pages, _ = collecter()
    # Toutes les pages réelles du harnais. Les endpoints de fragment en sont
    # écartés : ils n'ont pas de layout, donc pas de zone différée à réserver.
    FRAGMENTS = ("widget-", "rapport-mercenaires", "classement-detaille",
                 "admin-dashboard", "admin-resume", "auth-")
    cibles = [c for c in pages if not c.startswith(FRAGMENTS)]
    total_general = 0.0
    with sync_playwright() as p:
        nav = p.chromium.launch()
        for largeur, etiquette in ((1440, "desktop"), (390, "mobile")):
            print(f"\n══ {etiquette} ({largeur} px)")
            for nom in cibles:
                vide = geometrie(nav, pages[nom], largeur, bloquer=True)
                plein = geometrie(nav, pages[nom], largeur, bloquer=False)
                communs = set(vide) & set(plein)
                bouges = {k: plein[k] - vide[k] for k in communs
                          if abs(plein[k] - vide[k]) >= 1}
                # Le déplacement **distinct**, et non la somme sur les repères :
                # cent éléments qui descendent des mêmes 120 px, c'est un saut
                # de 120 px, pas de 12 000. Ce qu'on additionne, ce sont les
                # sauts de nature différente que subit un même point.
                distincts = sorted({round(abs(v)) for v in bouges.values()})
                cumul = sum(distincts)
                total_general += cumul
                print(f"   {nom:20} {len(bouges):3} repères, "
                      f"sauts distincts {distincts or '—'}, {cumul:5.0f} px")
        nav.close()
    print(f"\n  total général : {total_general:.0f} px")
    return 0 if total_general == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
