// Putting a card of the Deck in the hand during a duel, with nothing
// happening: no draw, no "added to the hand" event, no shuffle. A turn
// played again up to a point can then go on with one card more, as if it
// had been in the hand all along (the roles' test of an extender).
#include "card.h"
#include "duel.h"
#include "field.h"
#include "ocgapi.h"

extern "C" {

// The Deck card of `player` closest to the top whose code or original
// code is `code`, moved to the hand. 0 if there is none.
int df_deck_to_hand(OCG_Duel ocg_duel, uint8_t player, uint32_t code) {
	auto* pduel = static_cast<duel*>(ocg_duel);
	auto& field = *pduel->game_field;
	if(player > 1)
		return 0;
	auto& deck = field.player[player].list_main;
	for(auto it = deck.rbegin(); it != deck.rend(); ++it) {
		card* pcard = *it;
		if(pcard->data.code != code && pcard->data.alias != code)
			continue;
		const bool deck_check = field.core.shuffle_deck_check[player];
		const bool hand_check = field.core.shuffle_hand_check[player];
		field.remove_card(pcard);
		field.add_card(player, pcard, LOCATION_HAND, 0);
		pcard->current.position = POS_FACEDOWN_DEFENSE;
		field.core.shuffle_deck_check[player] = deck_check;
		field.core.shuffle_hand_check[player] = hand_check;
		return 1;
	}
	return 0;
}

}
