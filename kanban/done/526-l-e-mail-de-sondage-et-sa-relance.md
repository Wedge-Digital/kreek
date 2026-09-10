# L'e-mail de sondage et sa relance

**Priorité : haute — c'est lui qui fait l'intérêt de la fonction**
**Épic :** E16 — Sondage de présence
**Dépend de :** rien (les gabarits se rendent et se relisent seuls)
**Fichiers :** `assets/templates/emails/fr_FR/competition_presence_survey.html`,
`.../competition_presence_reminder.html`,
`src/app/competitions/io/email/notification_emails.rs`,
`src/app/competitions/domain/notification_delivery.rs`

## L'objectif

Les deux gabarits, leurs structs, et les deux types de notification. Rien n'est
encore expédié — c'est la 527.

## Un seul message par coach, N paires de boutons

```rust
pub struct PresenceSurveyEmail {
    pub app_url: String,
    pub coach_name: String,
    pub competition_name: String,
    pub competition_url: String,
    pub round_name: String,
    pub round_dates: String,
    pub deadline: String,
    pub equipes: Vec<EquipeLigneVm>,    // R1
}

pub struct EquipeLigneVm { pub team_name: String, pub yes_url: String, pub no_url: String }
```

R1 porte sur l'équipe, jamais sur le coach : un coach qui engage deux équipes
reçoit **un** e-mail à deux paires de boutons. Un e-mail par équipe multiplierait
les messages pour la même soirée, et le coach ne saurait pas lequel il a déjà
traité.

`equipes` est un `Vec` même pour un coach à une seule équipe — un champ scalaire
aurait obligé à un second gabarit dès la première ligue un peu vivante.

**Les URL arrivent construites.** Le gabarit ne les assemble pas : ce serait
mettre la forme du lien dans du HTML d'e-mail, hors de portée de tout test.

## Les conventions des quatre gabarits existants

Dégradé `#003049 → #555770`, logo en URL absolue servi en 200×81, `width` et
`height` en **attributs HTML** pour Outlook, tout le style **en ligne** — un
client mail ignore les feuilles externes.

Le vert du projet est assombri en `#4A7364` sur le bouton « Je serai là » :
`--green` porte 3,3:1 sous du blanc, sous le seuil pour du 16 px même en gras.
**Le token n'est pas touché** — le corriger à la source dépasse ce chantier.

## Deux `NotificationType`, pas un

```rust
PresenceSurvey    => "presence_survey"
PresenceReminder  => "presence_reminder"
```

`DeliveryKey` portant déjà `target_date`, une seule variante aurait
mécaniquement suffi. Deux valent mieux : le journal dit **ce qui** est parti et
pas seulement quand, et une relance envoyée le jour de l'ouverture cesse d'être
bloquée par une clé qu'elle partagerait avec l'envoi initial.

Les valeurs stockées sont écrites à la main et **figées par le test** qui protège
déjà les quatre autres : un renommage de variante réarmerait sinon en silence
toutes les notifications déjà envoyées.

## Trois écarts, décidés en la livrant

**`round_dates` n'existe pas ; les deux structs portent `date_start` et
`date_end: Option<String>`.** La carte annonçait une chaîne composée. Les quatre
e-mails plus anciens portent les deux dates séparées, et la maquette aussi : deux
lignes étiquetées, « Ouverture » devenant « Se tient le » sur une journée à date
fixe. Composer un libellé pour ce seul e-mail aurait été la divergence que
`CLAUDE.md` nomme au sujet des 2 075 kPo — une mise en forme inventée pour un
écran, et le seul écran qui se trompe.

**Un seul chemin de rendu, y compris pour le coach à une seule équipe.** La
maquette mettait alors « Ton équipe » dans la carte d'information, et ne répétait
l'intertitre que pour plusieurs. Le gabarit rend toujours la boucle : l'intertitre
porte la même information, et il n'y a qu'une branche à éprouver. La ligne « Ton
équipe » disparaît donc de la carte d'information.

**L'indication de bas de bloc dit une condition de plus que la maquette.**
« Tu pourras changer d'avis tant que le sondage est ouvert » est faux quand un
rapport de la journée est publié : R13 fige alors les présences, et le clic est
refusé sur une campagne pourtant ouverte — le cas d'une réouverture. La phrase
ajoute « et que la journée n'a pas commencé ».

## Ce que le compilateur a dit, et qui valait d'être écouté

Ajouter les deux variantes a rendu non exhaustifs les deux `match` du cron —
`notification_recipients.rs` et `notification_dispatch.rs`. Ce n'était pas une
formalité : ces deux types **ne passent par aucun des deux**. L'expédition part à
la demande depuis `launch_survey_use_case` via `ISurveyMailer`, et l'e-mail porte
une paire de boutons par équipe, ce que le patron « un destinataire, un message »
du dispatcher ne sait pas rendre.

Les deux bras rendent donc une liste vide et un couple vide, en le disant. Pas
d'`unreachable!()` : il ferait tomber le cron entier sur une donnée inattendue,
là où un sujet vide se verrait sans rien casser.

## Checklist

- [x] Les deux gabarits, sur le patron des quatre existants
- [x] `PresenceSurveyEmail`, `PresenceReminderEmail`, `EquipeLigneVm`
- [x] Les deux variantes, leurs valeurs ajoutées au test de figeage
- [x] Un test de rendu : deux équipes produisent quatre boutons, aux quatre URL
- [x] Les deux nouveaux gabarits entrent dans les deux contrôles de famille —
      `aucune_variable_ne_se_rend_vide` et `aucune_classe_utilisee_n_a_perdu_sa_regle`
- [ ] Relecture du rendu dans un client mail réel avant de clore la carte
      — les trois rendus sont produits, la relecture reste à faire
- [x] `make lint`, `make check-arch`, `make test`
