# E24 — La navigation mobile

**État :** `to_be_refined` — 3 cartes à écrire. Découpage validé le 2026-11-01.
Née le 2026-11-01 des maquettes
mobiles. **Se livre après E25** (décision du 2026-11-01) : l'onglet « Matchs »
mène à « Mes matchs » dès sa sortie.

**Maquettes :** la barre d'onglets et l'en-tête de tous les écrans
`app-mobile-*.html` (`app-mobile-shell.js`).

## La fonction

La barre d'onglets mobile (`app-menu.html`) ne mène pas où l'on croit :

| Onglet | Destination aujourd'hui |
|---|---|
| 🏆 Compétitions | la liste des compétitions |
| 📋 Match | le formulaire de match hors calendrier |
| 🛡️ Équipes | **la création d'une équipe** |
| 📊 Mes équipes | la liste de ses équipes |
| 👤 Compte | **la déconnexion** |

Il n'y a pas d'onglet Accueil : sur mobile, on ne revient au fil d'actualité
par aucun chemin.

La cible : **Accueil · Équipes · Matchs · Ligue · Moi**. « Équipes » ouvre la
recherche (E26), « Matchs » la page « Mes matchs » (E25) — page sans équivalent
sur ordinateur, que seule la barre mobile référence —, « Ligue » les
compétitions. Créer une équipe, changer d'espace et se déconnecter passent dans
« Moi ». **L'administration reste sur ordinateur** (décision du 2026-11-01).

L'onglet « 📋 Match » (formulaire hors calendrier) disparaît : sur mobile,
« Nouveau match » vit dans « Mes matchs », qui reprend sa règle de masquage.

La barre latérale de l'ordinateur ne change pas : la barre mobile est un
balisage à part, que le CSS bascule sous 768 px.

## Les cartes

| # | Intitulé | Apport |
|---|---|---|
| — | La barre d'onglets mobile | les cinq onglets et la section active Accueil (`ActiveSection` n'en a pas) ; « Équipes » vers « Mes équipes » tant que E26 n'est pas livrée ; le 👤 de l'en-tête ne déconnecte plus |
| — | La page « Moi » | créer une équipe, changer d'espace, se déconnecter ; pas d'administration |
| — | Les badges | des compteurs calculés : matchs à jouer, évolutions à faire |

## Ce qui commande l'ordre

E25 d'abord : « Matchs » a sa page dès la sortie de la barre. « Équipes »
pointe vers « Mes équipes » jusqu'à E26. La page « Moi » suit la barre ; les
badges viennent en dernier.

## Conséquences sur l'existant

**Aucune donnée** n'est lue autrement ni écrite. Les effets portent sur des
habitudes et sur des tests :

- la déconnexion quitte l'onglet « Compte » et le 👤 de l'en-tête pour « Moi » ;
  la création d'équipe aussi ;
- deux tests (`test_recruitment_phase`, `test_dismissals_phase`) vérifient que
  leur panier fixe ne chevauche pas la barre mobile : la barre garde sa
  hauteur, ou ils sont repris ;
- le volet mobile de `test_competition_hors_calendrier` cherche l'onglet
  « Match » : il est repris par E25, qui porte désormais la règle ;
- les badges coûtent une ou deux lectures par navigation, le menu étant
  rechargé à chaque page.

## Ce que l'épic ne couvre pas

- **La cloche de notifications** des maquettes : les notifications sont
  éphémères (E23), il n'y a rien à compter.
- **La barre latérale de l'ordinateur**, ni son entrée hors calendrier.
- **L'administration sur mobile.**

## Terminé quand

Sur un téléphone, les cinq onglets mènent chacun à sa page, l'onglet actif est
marqué sur chacune, et l'on se déconnecte depuis « Moi » sans qu'un onglet le
fasse par surprise. L'ordinateur n'a pas changé.
