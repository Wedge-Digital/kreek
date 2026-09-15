# Le test qui traverse toute la chaîne du sondage

**Priorité : haute — c'est lui qui autorise à clore E16**
**Épic :** E16 — Sondage de présence
**Dépend de :** rien (les 27 cartes de l'épic sont livrées)
**Fichiers :** `tests/e2e/test_presence_chaine_complete.py` *(nouveau)*,
`kanban/epics/en_cours/E16-sondage-de-presence.md`,
`src/app/competitions/io/web/admin/presences_widgets.rs`,
`src/app/competitions/use_cases/presences/confirm_draw_use_case.rs`,
`src/app/competitions/use_cases/presences/propose_repair_use_case.rs`

## Pourquoi cette carte existe

Les vingt-sept cartes d'E16 sont en `done/`, et l'épic est toujours dans
`en_cours/`. Son propre texte ne sait plus où il en est : l'en-tête annonce
« 20 faites » et « Restent l'e-mail, son expédition et les e2e (526 à 528) »
— les trois sont faites — pendant que la section *État* dit « Treize cartes
faites sur vingt-sept ».

Mais le décompte n'est pas le sujet. E16 se termine sur un **critère
observable**, et c'est lui qui n'est pas prouvé :

> Un organisateur ouvre une campagne sur une journée, reçoit des réponses, tire
> au sort et retrouve ses rencontres au Calendrier — **sans avoir saisi une
> seule présence à la main**.

## Trente-sept tests, et la chaîne coupée en deux

| Fichier | Ce qu'il prouve | Où il s'arrête |
|---|---|---|
| `test_presence_reponse_publique.py` | le coach répond par son lien, l'organisateur le voit | **avant le tirage** |
| `test_competition_presences.py::test_j4_tirer_puis_valider…` | le tirage écrit les rencontres au Calendrier | ses présences viennent de `_tous_presents()`, qui poste sur l'endpoint **`answer` de l'administration** |

Les deux moitiés ne se touchent pas. Celle qui va jusqu'au Calendrier saisit les
présences à la main — précisément ce que le critère exclut, et précisément ce
que l'épic dit ne rien résoudre : « un onglet où l'organisateur coche quatorze
cases lui-même ne résout rien qu'il ne sache déjà faire ».

**Aucun test ne traverse.** « Toutes les cartes sont dans `done/` » n'est pas une
preuve que la fonction marche — l'épic l'écrit elle-même.

## Ce que le test fait, et ce qu'il ne peut pas faire

Aucune intervention humaine n'est nécessaire : **le jeton se lit en base**, et
`test_presence_reponse_publique.py` le fait déjà.

```python
jeton = _jetons(round_id)[0]
page.goto(_lien(jeton, "oui"), wait_until="load")
```

Visiter cette URL, c'est exactement ce que fait le coach en cliquant dans sa
boîte mail. Le parcours complet est donc automatisable de bout en bout :

```
lancer la campagne
  → lire les N jetons en base
  → visiter le lien « oui » de chacun          ← aucun appel à `answer`
  → clore
  → tirer
  → valider
  → vérifier les rencontres dans competition_match_day_pairings
```

**Ce qu'il ne prouve pas, et qu'il doit dire en commentaire :** qu'un e-mail est
réellement parti et arrivé. Le test part du jeton, pas de la boîte mail. Cette
moitié-là relève de la 527 et d'une relecture humaine — hors du périmètre de
cette carte.

### Le garde-fou qui fait la valeur du test

Il ne suffit pas de ne pas appeler `answer` : il faut que **rien** ne l'appelle.
Le test vérifie donc en base qu'aucune réponse ne porte de `saisi_par_admin` —
la colonne que R6 réserve à la saisie de l'organisateur, et que le chemin du
jeton laisse à `NULL`.

Sans cette assertion, un futur remaniement du fixture pourrait réintroduire une
saisie d'organisateur sans que le test bronche, et il recommencerait à prouver
la moitié qu'on prouve déjà.

## Les quatre imports morts

Ils sortent à chaque `cargo check` depuis la vague 3 :

```
presences_widgets.rs:36         EquipeSollicitee
confirm_draw_use_case.rs:41     PresenceSurvey
propose_repair_use_case.rs:31   MatchDay
propose_repair_use_case.rs:42   TeamInfoDto
```

Rangés ici parce qu'ils sont dans le code des présences et qu'un avertissement
qu'on croise sans le lire est un avertissement qui finit par en cacher un vrai.

## La remise à jour de l'épic

Le texte d'E16 est réécrit pour dire son état réel : les vingt-sept cartes
livrées, ce que ce test ajoute, et **ce qui reste** — la relecture des trois
e-mails dans un client réel, case non cochée de la carte 526 alors qu'elle est
en `done/`.

**L'épic ne change pas de dossier.** Elle reste dans `en_cours/` : sa clôture
demande une validation humaine de la fonction entière, qui n'appartient pas à
cette carte.

## Terminé quand

`tests/e2e/test_presence_chaine_complete.py` passe, et son scénario ne contient
aucun appel à l'endpoint `answer` de l'administration — vérifiable par
`grep -c '"answer"'` sur le fichier, qui doit rendre 0.

## Tests

- **E2E** — le scénario ci-dessus, en un seul test. Falsifié en le faisant
  partir de `_tous_presents()` : il doit alors échouer sur l'assertion
  `saisi_par_admin`.
- **Pas de test unitaire.** Tout ce que cette carte ajoute est un parcours ; les
  couches qu'il traverse sont déjà couvertes par les vingt-sept cartes livrées.

## Checklist

- [ ] `test_presence_chaine_complete.py`, un scénario, aucun appel à `answer`
- [ ] L'assertion `saisi_par_admin IS NULL` sur toutes les réponses
- [ ] Le commentaire qui dit ce que le test ne prouve pas
- [ ] Les quatre imports morts retirés
- [ ] Le texte d'E16 remis à jour, l'épic laissée dans `en_cours/`
- [ ] `make lint`, `make check-arch`, `make test`, les 4 fichiers e2e des présences
