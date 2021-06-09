#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::{DispatchError, DispatchResult},
    ensure,
    traits::Randomness,
    RuntimeDebug, StorageDoubleMap, StorageValue,
};

use frame_system::ensure_signed;
use sp_io::hashing::blake2_128;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq)]
pub enum KittyGender {
    Male,
    Female,
}

#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq)]
pub struct Kitty(pub [u8; 16]);

pub trait Config: frame_system::Config {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
}

decl_storage! {
    trait Store for Module<T: Config> as Kitties {
        /// Stores all the kitties, key is the kitty id
        pub Kitties get(fn kitties): double_map hasher(blake2_128_concat) T::AccountId, hasher(blake2_128_concat) u32 => Option<Kitty>;
        /// Stores the next kitty ID
        pub NextKittyId get(fn next_kitty_id): u32;
    }
}

decl_event! {
    pub enum Event<T> where
        <T as frame_system::Config>::AccountId,
    {
        /// A kitty is created. \[owner, kitty_id, kitty\]
        KittyCreated(AccountId, u32, Kitty),
        /// A new kitten is bred. \[owner, kitty_id, kitty\]
        KittyBred(AccountId, u32, Kitty),
    }
}

decl_error! {
    pub enum Error for Module<T: Config> {
        KittiesIdOverflow,
        InvalidKittyId,
        SameGender,
    }
}

impl Kitty {
    pub fn gender(&self) -> KittyGender {
        if self.0[0] % 2 == 0 {
            KittyGender::Male
        } else {
            KittyGender::Female
        }
    }
}
impl<T: Config> Module<T> {
    /// I don't like this b/c if later operations FAIL the NextKittyId will still be mutated
    // pub fn get_next_kitty_id() -> sp_std::result::Result<u32, DispatchError> {
    // let current_id = Self::next_kitty_id();
    // let next_id = current_id
    // .checked_add(1)
    // .ok_or(Error::<T>::KittiesIdOverflow)?;
    // NextKittyId::put(next_id);

    // Ok(current_id)
    // }

    pub fn random_payload(sender: &T::AccountId) -> [u8; 16] {
        // Generate a random 128bit value
        let payload = (
            <pallet_randomness_collective_flip::Module<T> as Randomness<T::Hash>>::random_seed(),
            sender,
            <frame_system::Module<T>>::extrinsic_index(),
        );

        payload.using_encoded(blake2_128)
    }
}

decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        #[weight = 1000]
        pub fn breed(origin, kitty_id_1: u32, kitty_id_2: u32) {
            let sender = ensure_signed(origin)?;

            NextKittyId::try_mutate(|next_id| -> DispatchResult {
                let current_id = *next_id;
                *next_id = next_id.checked_add(1).ok_or(Error::<T>::KittiesIdOverflow)?;

                let kitty_1 = Self::kitties(&sender, kitty_id_1).ok_or(Error::<T>::InvalidKittyId)?;
                let kitty_2 = Self::kitties(&sender, kitty_id_2).ok_or(Error::<T>::InvalidKittyId)?;

                ensure!(kitty_1.gender() != kitty_2.gender(), Error::<T>::SameGender);

                let kitty_dna_1 = kitty_1.0;
                let kitty_dna_2 = kitty_2.0;

                let selector = Self::random_payload(&sender);

                let mut new_dna = [0u8; 16];

                for i in 0..kitty_dna_1.len() {
                    new_dna[i] = if selector[i] == 0 { kitty_dna_1[i] } else { kitty_dna_2[i]};
                }

                let kitty = Kitty(new_dna);
                Kitties::<T>::insert(&sender, current_id, kitty.clone());

                Self::deposit_event(RawEvent::KittyBred(sender, current_id, kitty));

                Ok(())
            })?;
        }

        /// Create a new kitty
        #[weight = 1000]
        pub fn create(origin) {
            let sender = ensure_signed(origin)?;

            NextKittyId::try_mutate(|next_id| -> DispatchResult {
                let current_id = *next_id;

                *next_id = next_id.checked_add(1).ok_or(Error::<T>::KittiesIdOverflow)?;

                let dna = Self::random_payload(&sender);

                // Create and store kitty
                let kitty = Kitty(dna);
                Kitties::<T>::insert(&sender, current_id, kitty.clone());

                // Emit event
                Self::deposit_event(RawEvent::KittyCreated(sender, current_id, kitty));

                Ok(())
            })?;
        }
    }
}
