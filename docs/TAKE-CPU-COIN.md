# Take the CPU's coin: L3 + B

On Training's fighter select, L3 + B puts the CPU's coin in your hand, so you can give the CPU another fighter
without first moving the hand onto its coin.

<video src="media/take-cpu-coin.mp4" autoplay muted loop playsinline controls></video>

*L3 + B takes the CPU's coin from Samus, L3 + Right four times carries it to Fox, and A places it. Eden,
2026-11-23, build `0353c5a`. The game shows no input on this screen, so the button labels are drawn on the clip
from the run's input log.*

## How to use it

1. Open Training's fighter select and place your own coin on your fighter.
2. Hold **L3** and press **B**. The CPU's coin leaves its fighter and comes to your hand.
3. Keep **L3** held and press the **D-pad** to move the coin, one fighter per press.
4. Press **A** to put the coin down on the fighter under it.

## Buttons

| Buttons | When | What happens |
| --- | --- | --- |
| L3 + B | your hand is empty | The CPU's coin comes to your hand |
| L3 + D-pad | a coin is in your hand | The coin moves one fighter in that direction |
| A | a coin is in your hand | The coin is placed on the fighter under it |

## When L3 + B does nothing

- **Your hand already holds a coin**, your own or the CPU's. Put it down first.
- **The CPU's coin cannot be taken at that moment**, for example while it is still moving to its place. L3 + B
  then does nothing at all; it never takes your own coin back in its place.

L3 + B also never backs out of the fighter select, however long you hold B. That stays so until you let go of
B, even if you let go of L3 first.

## Good to know

- Hold L3 first, then press B. A B pressed before L3 is the game's own B.
- B without L3 is unchanged: with an empty hand it takes your own coin back, and holding it backs out of the
  fighter select.

## Where it works

Training's fighter select only: the one with your coin and one CPU coin. On every other screen L3 + B is the
game's own B. That includes the ordinary Smash fighter select and a waiting room's fighter select; a waiting
room is not expected to carry a CPU coin, which has not been checked in a game.

## Checked so far

Build `0353c5a`, Eden, 2026-11-23: the take, moving and placing the coin, L3 + B doing nothing with a full hand
(a tap and a 120-frame hold) and Training loading with the placed fighter all passed
([EDEN-RESULTS.md](EDEN-RESULTS.md)). The clip shows the take, the move and the placing; the full-hand case was
checked in the same run and is not in the clip. Not tried on a console, and not together with Instant Back.
