# Les tests e2e de l'encart

**Priorité : haute — deux de ces scénarios ne se rencontrent pas à la main**
**Épic :** E16 — Sondage de présence
**Dépend de :** 531
**Fichiers :** `tests/e2e/test_competition_encart_presence.py`, `tests/impact-map.toml`

## Les scénarios

| Scénario | Ce qu'il éprouve |
|---|---|
| coach avec une équipe, campagne ouverte | l'encart apparaît, deux boutons |
| il répond, la page ne bouge pas | l'onglet ouvert reste ouvert, le classement n'est pas rechargé |
| il change d'avis | la bascule, sans rechargement de page |
| coach avec **deux** équipes | deux lignes, une réponse par équipe (R1) |
| coach sans équipe engagée | **rien du tout** — pas d'encart vide, pas de marge |
| campagne close entre l'affichage et le clic | la carte explicative, **pas un fragment vide** |
| après le tirage, il se décommande | R30 — la mention, et **aucun adversaire annoncé** |
| deux campagnes ouvertes | deux cartes, la plus proche échéance en premier |

## Les deux qui valent le plus

**La campagne close entre l'affichage et le clic.** Personne ne rencontre cette
course en développement, et son mode d'échec — un encart qui disparaît
silencieusement — **ressemble à un succès**. C'est la classe de défaut que la
carte 486 a payée : un refus rendant le même `200` qu'un succès, avec une CI
rouge trois runs sur quatre.

**Le coach sans équipe engagée.** Il vérifie une absence, ce qu'on oublie de
tester : que la page est *exactement* celle d'avant — pas seulement qu'aucun
texte n'apparaît, mais qu'aucune marge ne s'est ajoutée. C'est le `outerHTML` de
la carte 531 qui le garantit, et rien d'autre ne le prouverait.

## Le piège habituel

**`cliquer_quand_cable` sur les boutons de l'encart** : il est injecté par htmx,
donc la fenêtre où il est peint mais pas encore câblé s'y présente. Pas de
`sleep`.

## Checklist

- [ ] Le fichier de test
- [ ] Les huit scénarios
- [ ] `cliquer_quand_cable`, aucun `sleep`
- [ ] `tests/impact-map.toml`, **même commit**
- [ ] `make e2e` passe (serveur dev lancé par l'utilisateur)
- [ ] `make lint`, `make check-arch`, `make test`
