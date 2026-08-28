# Recruter un journalier

**Conception :** `00-conception.md` — quinze décisions issues du grilling du
2026-08-28, à lire avant les phases.
**Maquette :** `assets/rawpages/html/app-team-recruitment.html`

## La fonction

Un journalier qui a joué pour une équipe peut être recruté définitivement à la
phase de recrutement suivante — avec l'expérience et les améliorations qu'il a
gagnées pendant le match. Celui qu'on ne recrute pas est perdu.

## Le renversement qui commande tout

**Un journalier est un joueur dès le début du rapport de match**, pas à son
recrutement. Il naît dans `players` avec un `membership: Journeyman`, joue,
gagne des SPP, prend ses améliorations comme les autres. Le recrutement ne fait
que basculer son `membership` en `Active`.

Sans ce renversement, les hausses de valeur du LRB seraient impossibles à
porter : un joueur qui n'existe pas ne peut pas s'améliorer.

## Les pages

| Page | État |
|---|---|
| `ecran-de-recrutement/` | phases 1 et 2 faites |
