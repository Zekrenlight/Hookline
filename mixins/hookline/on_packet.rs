fn hookline_packet(
    game: &mut crate::game::Game<'_>,
    packet: &horizon_client::mod_api::ClientboundPlayPacket,
) {
    use crate::game::hookline as hook;
    hook::on_splash(game, packet);
}
