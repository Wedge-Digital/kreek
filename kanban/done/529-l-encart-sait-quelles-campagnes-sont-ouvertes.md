# L'encart sait quelles campagnes sont ouvertes

**Priorité : moyenne — l'encart ne peut rien afficher sans elle**
**Épic :** E16 — Sondage de présence
**Dépend de :** 510, qui crée les tables et le port
**Fichiers :** `src/app/competitions/domain/presence_survey_repository_port.rs`,
`src/app/competitions/io/repository/presence_survey_repository.rs`,
`.../sql/presences/list_open_surveys_for_season.sql`
**Spec :** `docs/specs/sondage-presence/encart-competition/07-integration.md`

## L'objectif

```rust
async fn list_open_surveys_for_season(&self, season_id: &str, maintenant: &str)
    -> Result<Vec<PresenceSurvey>, PresenceSurveyRepositoryError>;
```

Une requête pour la saison, jamais une par journée — l'encart doit connaître
toutes les campagnes ouvertes, et les chercher une à une ferait vingt
allers-retours à chaque affichage de la page de détail.

## Elle rend des agrégats, pas un DTO de lecture

L'encart a besoin de `statut()`, de la réponse de chaque équipe et de son
horodatage : trois questions du domaine. Un DTO les aurait aplaties, et le VM
aurait dû les reconstituer — exactement ce que « un view model transpose, il ne
dérive pas » interdit.

## Le filtre vit à deux endroits, et c'est assumé

Le SQL **élague** — échéance passée, `close_le` renseigné — puis `statut()`
**décide**. R23 fait de la clôture un calcul, donc la règle est en principe au
domaine seul.

L'alternative serait de charger toutes les campagnes de la saison pour en jeter
la plupart, à chaque affichage d'une page que la majorité des visiteurs ouvre
sans être concernée.

**Ce qui rend le compromis sûr** : la référence reste le domaine, et un désaccord
entre les deux ne produit qu'une campagne chargée pour rien — **jamais une
campagne affichée à tort**. L'asymétrie est ce qui autorise l'élagage.

## Deux choses décidées en l'écrivant

**Deux requêtes, quel que soit le nombre de campagnes.** `find_by_round` et
`find_by_token` appellent tous deux `lire_les_reponses`, **une requête par
campagne**. Boucler dessus aurait fait `1 + N` allers-retours — exactement ce que
« une requête pour la saison, jamais une par journée » voulait éviter, déplacé
d'un cran. D'où `find_answers_by_surveys.sql`, le pendant pluriel du singulier
existant, et un regroupement par `survey_id` en mémoire. C'est le seul fichier que
cette carte n'annonçait pas.

L'agrégat, lui, passe par le **même `rehydrater`** que les deux lectures
unitaires : un troisième assemblage aurait été libre de diverger des deux
premiers sans que rien ne le dise.

**`>=` et non `>`.** `statut_de` ferme sur `aujourd_hui > deadline` : une campagne
échéant le 10 répond encore le 10. Un `>` dans le SQL l'écarterait — et c'est le
**seul sens dangereux** de ce compromis. L'asymétrie qui autorise l'élagage ne
joue que dans un sens : trop large, une campagne est chargée pour rien et le
domaine l'écarte ; trop étroit, elle n'arrive jamais et le domaine n'a plus rien à
rattraper.

## Les deux tests en plus, et la preuve qu'ils servent

Chacun a été éprouvé par un sabotage volontaire, puis le code restauré.

| Sabotage | Ce qui tombe |
|---|---|
| `deadline > $2` au lieu de `>=` | `une_campagne_echeant_aujourd_hui_repond_encore` — **et rien d'autre** |
| chaque campagne reçoit toutes les réponses | `les_reponses_ne_se_melangent_pas_entre_campagnes` — **et rien d'autre** |

Les quatre tests de la checklist passent dans les deux cas. Sur une requête qui
fait disparaître les campagnes le jour de leur échéance — le dernier jour où un
coach répond, donc le plus fréquenté — ils auraient tous été verts. Et
`une_campagne_ouverte_revient_avec_toutes_ses_reponses` reste vert sous le second
sabotage, ce qui était prévisible : avec une seule campagne, « toutes les
réponses » et « les siennes » sont la même chose.

C'est la mesure de ce que vaut une checklist écrite avant le code : elle couvre ce
qu'on savait alors, pas ce que l'écriture révèle.

## Checklist

- [x] La méthode au port et au dépôt
- [x] `list_open_surveys_for_season.sql` **et** `find_answers_by_surveys.sql`,
      sous `sql/presences/`
- [x] Les deux doubles du port l'implémentent — `FauxSurveyRepo` en refaisant le
      filtre par `statut()`, et non en rendant une liste vide qui aurait fait
      passer n'importe quel use case de l'encart sans voir une seule campagne
- [x] Sept tests d'intégration (vraie `PgPool`) : les quatre de la checklist, plus
      l'échéance du jour même, le non-mélange des réponses, et la saison sans
      campagne ouverte
- [x] Les deux tests ajoutés éprouvés par sabotage : chacun tombe seul
- [x] `make lint`, `make check-arch`, `make test`
