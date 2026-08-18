# Spike : RME Babyface × Raspberry Pi

> Objectif : valider que la Babyface est utilisable comme interface audio de ttySeq
> sur Raspberry Pi — détection, multicanal, stabilité, latence. Timeboxé : si ça
> dépasse une demi-journée, on note où ça bloque et on repasse au code (le
> développement de l'engine ne dépend pas de ce résultat).

## Contexte et risques connus

- **Pas de driver RME sous Linux.** La Babyface ne fonctionne sur Pi qu'en mode
  **Class Compliant (CC)**. La procédure d'activation et les capacités exposées en
  CC (nombre de canaux, sample rates) **dépendent du modèle** (Babyface 2010,
  Babyface Pro, Pro FS) — vérifier le manuel RME du modèle exact et la version de
  firmware avant tout.
- **Pas de TotalMix sous Linux.** Le routing/mixage interne doit être soit
  pré-configuré, soit piloté par les contrôles en façade (selon modèle). À évaluer :
  est-ce compatible avec un usage scène sans ordinateur de config ?
- **Alimentation.** Le budget USB du Pi est limité ; une interface bus-powered peut
  être instable. Prévoir l'alimentation externe de la Babyface si le modèle le
  permet, ou un hub USB alimenté.
- ttySeq visera 48 kHz / buffer 512-1024 sur Pi (spec §7) : c'est la configuration
  à valider en priorité.

## Prérequis

- Modèle et firmware identifiés, mode CC activé (procédure faite depuis macOS).
- Raspberry Pi avec OS à jour ; `alsa-utils` installés.
- Le binaire du spike audio cpal (chemin temps réel déjà validé sur macOS).

## Protocole

Remplir la colonne résultat à chaque étape.

| # | Étape | Commande / action | Attendu | Résultat |
|---|---|---|---|---|
| 1 | Détection | `aplay -l`, `cat /proc/asound/cards` | La Babyface apparaît comme carte ALSA | |
| 2 | MIDI | `amidi -l` | Les ports MIDI de la Babyface apparaissent | |
| 3 | Stéréo de base | `speaker-test -D hw:CARD=<nom> -c 2 -r 48000 -f S32_LE` | Son propre sur les sorties principales | |
| 4 | Canaux exposés | `speaker-test -c <N>` en montant N | Nombre de canaux réellement accessibles en CC (noter : sorties casque/lignes/ADAT) | |
| 5 | Sample rates | tester 44.1 / 48 / 96 kHz | Rates acceptés en CC | |
| 6 | Spike cpal | lancer le spike à buffer 1024, puis 512, 256, 128 | Pas de xrun audible ; noter le CPU (`top`) à chaque palier | |
| 7 | Stress | lecture continue 30 min à la config cible (48 kHz / 512) | Zéro xrun (`dmesg`, compteurs ALSA) | |
| 8 | Hotplug | débrancher/rebrancher en cours de route | La carte réapparaît proprement (comportement à documenter pour la spec hotplug §3.3.7) | |
| 9 | (Option) Latence round-trip | JACK + `jack_iodelay` avec câble loopback sortie→entrée | Mesure réelle, à comparer à l'objectif < 10 ms | |

## Critères de succès

Le spike est concluant si : détection fiable, **au moins 4 canaux de sortie**
utilisables (stéréo main + click séparé), 48 kHz / buffer ≤ 512 stable 30 min
sans xrun sur le spike cpal.

## Sorties du spike

- Grille ci-dessus remplie, versionnée dans ce fichier.
- Décision : la Babyface est-elle l'interface de référence sur Pi, ou faut-il
  prévoir/recommander une alternative class-compliant ?
- Retombées spec éventuelles : contraintes CC à documenter (canaux, routing sans
  TotalMix), comportement hotplug observé.
