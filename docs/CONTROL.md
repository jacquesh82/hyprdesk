# Socket de commande de hyprdesk

*[SDK](SDK.md) · [README](../README.fr.md)*

Le démon hyprdesk écoute sur un socket Unix. Le CLI s'en sert
(`hyprdesk toggle`, `hyprdesk settings`…), et n'importe quel programme peut
faire de même : un script, un raccourci, un autre démon, un agent IA.

Référence tirée de `src/control.rs`. Elle décrit le protocole tel qu'il est
aujourd'hui : **texte, non versionné** (voir §6).

## 1. Emplacement

```
$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/hyprdesk.sock
```

C'est le dossier d'instance de Hyprland. Le socket est donc propre à la
session graphique, et un programme lancé hors de Hyprland (une session
SSH, une unité systemd sans l'environnement de la session) doit connaître
`HYPRLAND_INSTANCE_SIGNATURE` pour le trouver. Le démon supprime un socket
orphelin au démarrage avant de le recréer.

## 2. Échange

Une connexion porte **une** requête et **une** réponse :

1. le client écrit **une ligne** terminée par `\n` ;
2. le démon répond par une ligne terminée par `\n`, puis ferme.

| Contrainte | Valeur |
|---|---|
| Délai pour envoyer la ligne | 2 s, sinon la connexion est fermée sans réponse |
| Ligne sans `\n` final | ignorée, fermée sans réponse |
| Taille d'une ligne | 256 Kio au plus (`MAX_LINE`) |
| Connexions simultanées | 32 ; au-delà, la connexion est refermée sans réponse |
| Délai côté démon pour accepter la commande | 2 s |

Les espaces en début et en fin de ligne sont ignorés. Un argument qui peut
contenir un saut de ligne (`problem`, `ask`) voyage en **JSON** pour tenir
sur une seule ligne.

## 3. Réponses

| Réponse | Sens |
|---|---|
| `ok` | Commande **acceptée** dans la file de l'interface. Ça ne veut pas dire qu'elle est déjà exécutée, ni qu'elle a réussi. |
| `démon occupé` | La file n'a pas pu la prendre en 2 s. Réessayer plus tard. |
| `commande inconnue : <ligne>` | Ligne non reconnue, ou argument JSON invalide. |
| `commande trop longue (262144 octets au plus)` | Ligne au-delà de `MAX_LINE`. |
| *(texte libre)* | Réponse de `status` (§4.6). |

Les réponses autres que `ok` sont **traduites** dans la langue de
l'interface. Un client ne doit tester que `ok`, puis traiter tout le reste
comme un échec à montrer tel quel.

## 4. Commandes

Les alias donnent la même commande.

### 4.1 Modes du bureau

| Ligne | Alias | Effet |
|---|---|---|
| `toggle` | | full-desktop, ou no-desktop s'il y est déjà |
| `show` | `full-desktop` | plein bureau, fenêtres mises de côté |
| `hide` | `no-desktop` | bureau retiré |
| `launcher-only` | `controls` | fenêtres en place, lanceur (dock) accessible |
| `toggle-launcher` | | launcher-only, ou no-desktop s'il y est déjà |
| `cycle` | | no-desktop → launcher-only → full-desktop → no-desktop |
| `forget-window-places` | | oublie les places des fenêtres choisies en plein bureau |

### 4.2 Surfaces

| Ligne | Effet |
|---|---|
| `settings` | ouvre les Réglages sur `desktop` |
| `settings <paquet>[/<onglet>]` | ouvre les Réglages sur un paquet (`screens`, `fond`, `bureau/raccourcis`…) |
| `settings ?<mots>` | ouvre les Réglages sur la recherche |
| `toggle-settings` | ouvre les Réglages, ou les referme s'ils sont ouverts sur cet écran |
| `focus <surface>` | donne le clavier à une surface du shell : `bar` (barre de l'écran actif), `dock`, `notifications` (centre), `settings` (volet des Réglages, sur son premier champ) ; rien ne se referme, le mode du bureau ne change pas (raccourcis `keys.focus_*`) |
| `force-quit` | ouvre la boîte « Forcer à quitter » — une ligne par application, à confirmer en deux temps, puis ses fenêtres sont tuées —, ou la referme (raccourci `keys.force_quit`) |
| `keyboard-access` | bascule l'accès clavier complet — Tab atteint chaque commande des Réglages, ou seulement les champs de saisie — et l'annonce à l'écran (raccourci `keys.keyboard_access`, réglage `keyboard_access`) |
| `menu <module>` | ouvre le menu d'un module de la barre (`volume`, `clock`, `network`, `tray`, `session`… ; anciens noms admis : `reseau`, `batterie`…) |
| `search` | lanceur avec recherche (ouvre ou referme) |
| `files` | recherche de fichiers (`:f`) |
| `clipboard` (alias `clip`) | historique du presse-papiers (`:clip`) |
| `apps` | grille des applications, plein écran |
| `overview` | vue d'ensemble des fenêtres |
| `switch` (alias `switch next`) | sélecteur de fenêtres : ouvre, ou fenêtre suivante |
| `switch prev` (alias `switch-prev`) | fenêtre précédente |
| `assistant` | ouvre ou referme LYRA |
| `tour` | Réglages, sur la visite guidée |
| `wizard` | assistant de configuration |
| `color-test` | mire plein écran des couleurs (ouvre ou ferme) |

### 4.3 Panorama, notifications, trousseau

| Ligne | Effet |
|---|---|
| `panorama` | équivaut à `panorama toggle` |
| `panorama <toggle\|show\|hide>` | affiche ou escamote le Panorama des écrans bandeau |
| `panorama page <n\|id>` | va à une page |
| `panorama detect` | relit les écrans et refait les propositions |
| `notifications dnd [on [1h\|soir] \| off]` | Ne pas déranger : jusqu'à désactivation, une heure (`1h`) ou jusqu'à ce soir, 18 h (`soir`, refusé le soir) |
| `notifications clear` | efface les notifications |
| `mymfa unlock` | carte de déverrouillage du trousseau myMFA (argument inconnu : même effet) |
| `mymfa lock` | verrouille le trousseau |
| `mymfa open` | fenêtre complète du trousseau |
| `mymfa setup` | création du coffre |
| `mymfa codes` / `mymfa logins` | lanceur sur les codes 2FA (`:2fa`) / les mots de passe (`:mdp`) |

Les autres sous-commandes du CLI (`panorama layout`, `panorama video`,
`notifications list`…) sont décrites par `hyprdesk --help`. Passer par le CLI
pour celles-là.

### 4.4 LYRA et problèmes

| Ligne | Effet |
|---|---|
| `ask ["<titre>","<texte>"]` | nouvelle conversation LYRA, affichée aussitôt |
| `problem "<texte>"` | problème confié à LYRA (conversation signalée par une notification), sinon fenêtre de détail copiable |
| `problem-show "<texte>"` | le même, affiché aussitôt |

L'argument est du JSON : un tableau de deux chaînes pour `ask`, une chaîne
pour `problem`. Un texte de plusieurs lignes tient donc sur une seule ligne
de protocole, dans la limite de 256 Kio (assez pour joindre la fin d'un
journal).

### 4.5 Écrans et cycle de vie

| Ligne | Effet |
|---|---|
| `screens-off` | éteint les écrans, la machine reste allumée ; un changement de volume les rallume |
| `reload` | relit la configuration |
| `quit` | arrête le démon (code de sortie 75 : systemd ne le relance pas) ; l'expéditeur est tracé dans le journal |

### 4.6 `status`

`status` est servi sans passer par l'interface. Il répond toujours, même
quand la file est pleine. La réponse est un résumé **lisible, traduit et non
stable** :

```
full-desktop (pid 4242) — écrans DP-3:1 HDMI-A-1:2 — config ok (~/.config/hypr/hyprdesk.lua)
agent polkit : …
```

On peut s'en servir pour savoir si le démon répond, mais **ne pas
l'analyser** : son format changera quand l'état du bureau sera publié (§6).

### 4.7 Lignes internes

Envoyées par hyprdesk à lui-même. Elles sont documentées pour qu'on les
reconnaisse dans le journal, pas pour être appelées :

| Ligne | Émise par |
|---|---|
| `osd-show <genre> <valeur> <muet>` | `hyprdesk osd`, une fois le volume ou la luminosité réglés |
| `defaults <composant\|all>` | `hyprdesk config reset`, pour que le démon relise son état |

## 5. Exemples

**Shell** (`socat`, ou `nc -U -N` d'OpenBSD netcat) :

```sh
sock="$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/hyprdesk.sock"
printf 'settings screens\n' | socat - UNIX-CONNECT:"$sock"
printf 'ask %s\n' "$(jq -cn --arg t 'Disque' --arg x "$(df -h /)" '[$t,$x]')" \
  | socat - UNIX-CONNECT:"$sock"
```

Le plus simple reste souvent le CLI, qui fait le même échange et rend un
code de sortie : `hyprdesk settings screens`, `hyprdesk ask Disque "$(df -h /)"`.

**Python** :

```python
import json, os, socket

def hyprdesk(line: str) -> str:
    path = os.path.join(os.environ["XDG_RUNTIME_DIR"], "hypr",
                        os.environ["HYPRLAND_INSTANCE_SIGNATURE"], "hyprdesk.sock")
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as s:
        s.settimeout(5)
        s.connect(path)
        s.sendall(line.encode() + b"\n")
        return s.makefile().readline().strip()

assert hyprdesk("toggle") == "ok"
hyprdesk("ask " + json.dumps(["Sauvegarde", "Échec de restic :\n…"]))
```

**Rust** : `std::os::unix::net::UnixStream`, écrire la ligne, lire jusqu'à
la fin. C'est ce que fait `control::send`, qui n'est pas encore exposé dans
une crate cliente (§6).

## 6. Limites connues

- **Pas de version** : aucune commande ne dit quelle version du protocole
  parle le démon. `hd.version` (en Lua) et `hyprdesk --version` donnent la
  version de hyprdesk.
- **Pas d'événements** : on ne peut pas s'abonner aux changements de mode ou
  d'écran. En attendant, les changements de workspace et de fenêtre se
  suivent sur le socket d'événements de Hyprland
  (`…/$HYPRLAND_INSTANCE_SIGNATURE/.socket2.sock`).
- **Réponses en texte traduit**, et `status` sans format machine.
- **Pas de crate cliente** pour ce socket. Celle du thème
  (`hyprdesk-theme-client`) ne couvre que la palette.

Ces points sont repris dans la feuille de route du [SDK](SDK.md#9-pas-encore-disponible).
