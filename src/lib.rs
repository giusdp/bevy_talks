use bevy::prelude::{App, Plugin};

// use prelude::*;

pub mod errors;
pub mod events;
pub mod loader;
pub mod prelude;
pub mod screenplay;
pub mod types;

pub struct TalksPlugin;

impl Plugin for TalksPlugin {
    fn build(&self, app: &mut App) {
        // app.add_event::<RequestNextActionEvent>()
        //     .add_event::<TalkActionEvent>()
        //     .add_event::<EnterActionEvent>()
        //     .add_event::<ExitActionEvent>()
        //     .add_event::<ChoiceActionEvent>()
        //     .add_event::<NextActionErrorEvent>()
        //     .add_event::<ChoicePickedEvent>()
        //     .add_event::<ChoicePickedErrorEvent>()
        //     .add_asset::<Screenplay>()
        //     .init_asset_loader::<ScreenplayLoader>()
        //     .add_system(advance_play)
        //     .add_system(choice_picked);
    }
}

// fn advance_play(
//     mut ev_next_action: EventReader<RequestNextActionEvent>,
//     mut assets: ResMut<Assets<Screenplay>>,
//     mut ev_talk_w: EventWriter<TalkActionEvent>,
//     mut ev_enter_w: EventWriter<EnterActionEvent>,
//     mut ev_exit_w: EventWriter<ExitActionEvent>,
//     mut ev_choice_w: EventWriter<ChoiceActionEvent>,
//     mut ev_err_w: EventWriter<NextActionErrorEvent>,
// ) {
//     for e in ev_next_action.iter() {
//         info!("Received RequestNextActionEvent");
//         if let Some(screenplay) = assets.get_mut(&e.0) {
//             let res = screenplay.next_action();
//             match res {
//                 Ok(_) => {
//                     info!("Moved to next action");
//                     notify(
//                         screenplay,
//                         &mut ev_talk_w,
//                         &mut ev_enter_w,
//                         &mut ev_exit_w,
//                         &mut ev_choice_w,
//                     );
//                 }
//                 Err(e) => {
//                     warn!("Error moving to next action: {:?}", e);
//                     ev_err_w.send(NextActionErrorEvent(e));
//                 }
//             }
//         }
//     }
// }

// fn choice_picked(
//     mut ev_choice_picked: EventReader<ChoicePickedEvent>,
//     mut assets: ResMut<Assets<Screenplay>>,
//     mut ev_talk_w: EventWriter<TalkActionEvent>,
//     mut ev_enter_w: EventWriter<EnterActionEvent>,
//     mut ev_exit_w: EventWriter<ExitActionEvent>,
//     mut ev_choice_w: EventWriter<ChoiceActionEvent>,
//     mut ev_err_w: EventWriter<ChoicePickedErrorEvent>,
// ) {
//     for e in ev_choice_picked.iter() {
//         info!("Received choice picked event to: {}", e.1);
//         if let Some(screenplay) = assets.get_mut(&e.0) {
//             let res = screenplay.jump_to(e.1);
//             match res {
//                 Ok(_) => {
//                     info!("Jumped to picked action");
//                     notify(
//                         screenplay,
//                         &mut ev_talk_w,
//                         &mut ev_enter_w,
//                         &mut ev_exit_w,
//                         &mut ev_choice_w,
//                     );
//                 }
//                 Err(e) => {
//                     warn!("Error picking choice: {:?}", e);
//                     ev_err_w.send(ChoicePickedErrorEvent(e));
//                 }
//             }
//         }
//     }
// }

// fn notify(
//     screenplay: &Screenplay,
//     ev_talk_w: &mut EventWriter<TalkActionEvent>,
//     ev_enter_w: &mut EventWriter<EnterActionEvent>,
//     ev_exit_w: &mut EventWriter<ExitActionEvent>,
//     ev_choice_w: &mut EventWriter<ChoiceActionEvent>,
// ) {
//     match screenplay.action_kind() {
//         ActionKind::PlayerChoice => {
//             info!("Current action is: choice");
//             ev_choice_w.send(ChoiceActionEvent(screenplay.choices().unwrap_or_default()));
//         }
//         ActionKind::ActorTalk => {
//             info!("Current action is: talk");
//             ev_talk_w.send(TalkActionEvent {
//                 actors: screenplay.actors().unwrap_or_default(),
//                 text: screenplay.text().to_string(),
//                 sound_effect: screenplay.sound_effect(),
//             });
//         }
//         ActionKind::ActorEnter => {
//             info!("Current action is: enter");
//             ev_enter_w.send(EnterActionEvent(screenplay.actors().unwrap_or_default()));
//         }
//         ActionKind::ActorExit => {
//             info!("Current action is: exit");
//             ev_exit_w.send(ExitActionEvent(screenplay.actors().unwrap_or_default()));
//         }
//     };
// }
