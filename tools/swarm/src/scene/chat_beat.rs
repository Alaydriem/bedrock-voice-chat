/// One line the script produces.
///
/// The two variants are separate frames on the wire and render differently: a `Chat` line
/// carries an author and reads as a person talking, an `Event` carries none and reads as
/// the server speaking. Collapsing them into one type with an optional author would let a
/// death message be attributed to whoever died.
pub enum ChatBeat {
    Chat { author: String, text: String },
    Event { text: String },
}
