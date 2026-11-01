# Les tests e2e du nouveau parcours

**Épic :** E21 — Le mode de saisie et l'après-match en séquence · **Ordre :** 9 · **Dépend de :** 631 à 638
**Conception :** `docs/specs/saisie-mobile/E21-cadrage.md` (§ 7)

## Objectif

Le « Terminé quand » de l'épic, rejoué en navigateur : depuis le calendrier, un
coach démarre un match en « Je saisis tout » sans passer par la sélection,
saisit les actions des deux camps depuis le journal commun, enchaîne le MVP du
domicile — tiré au sort parmi six — puis du visiteur, et publie. L'autre coach
ouvre le même match et le voit sans pouvoir y écrire. Un compte rendu commencé
avant la livraison se termine comme avant.

## Conception

Chaque carte a déjà migré les tests qu'elle cassait (règle de l'épic) ; cette
carte ajoute le parcours entier, qu'aucune ne voit d'un bout à l'autre.

- **`test_match_report_solo_entry.py`** : DevCoach démarre depuis le calendrier ;
  le membre simple (`X-Bypass-Auth-Profile: simple`), coach du visiteur, suit
  dans un second contexte de navigateur — aucun bouton d'écriture, la
  publication refusée, le journal à jour au rechargement.
- **Viewport mobile** sur le même parcours : les mêmes routes, aucune route
  propre au mobile.
- **Le compte rendu d'avant** : un flux écrit sans `EntryModeChosen`, avec deux
  MVP dans un camp, inséré en base comme le ferait l'existant, se termine et se
  publie.
- **Le helper** : `match_report_helpers.py` regroupe ce que les cartes ont
  ajouté (`record_mvp_api`, mode) ; les copies privées qui ont dû bouger y sont
  ramenées quand elles ne divergent plus.

## Données de production

Aucune.

## Checklist

- [ ] Le parcours « Je saisis tout » complet, ordinateur et viewport mobile
- [ ] L'autre coach en lecture seule, deux contextes
- [ ] Un compte rendu d'avant E21 se termine et se publie
- [ ] Copies privées des helpers ramenées au module commun quand c'est possible
- [ ] `tests/impact-map.toml` mis à jour
- [ ] `make e2e` vert
