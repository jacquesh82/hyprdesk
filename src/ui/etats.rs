//! Termes d'état de l'interface (HIG TYP-14) : un terme par état, dans les
//! deux langues. Un état qui se lit seul — badge, pastille, ligne d'état —
//! prend une majuscule initiale (HIG TYP-06). Dans une phrase, il reste en
//! minuscules (« raccourci désactivé »), comme dans les sorties en ligne de
//! commande et le contexte de l'assistant.

use crate::i18n::tr;

/// Un état affiché seul dans l'interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Etat {
    /// Ce qu'on a allumé : paquet, widget, raccourci.
    Active,
    /// Ce qu'on a éteint.
    Inactive,
    /// Composant de la session qui ne tourne pas.
    Arrete,
    /// Composant de la session abandonné après ses plantages.
    Abandonne,
    /// Widget dont le chargement a échoué.
    EnErreur,
    /// Paquet central, qu'on ne coupe pas.
    Coeur,
    Connecte,
    NonConnecte,
    Installe,
    Introuvable,
    Integre,
    Livre,
    Tiers,
}

impl Etat {
    /// Le terme français, clé du catalogue.
    pub const fn terme(self) -> &'static str {
        match self {
            Self::Active => "Activé",
            Self::Inactive => "Désactivé",
            Self::Arrete => "Arrêté",
            Self::Abandonne => "Abandonné (plantages)",
            Self::EnErreur => "En erreur",
            Self::Coeur => "Cœur",
            Self::Connecte => "Connecté",
            Self::NonConnecte => "Non connecté",
            Self::Installe => "Installé",
            Self::Introuvable => "Introuvable",
            Self::Integre => "Intégré",
            Self::Livre => "Livré",
            Self::Tiers => "Tiers",
        }
    }

    /// Le terme dans la langue de l'interface.
    pub fn libelle(self) -> &'static str {
        tr(self.terme())
    }

    /// Composant de la session en cours d'exécution : « Actif (pid n) ». Un
    /// processus qui tourne est actif, il n'est pas « activé » : le terme du
    /// composant en marche n'est pas celui d'un réglage allumé.
    pub fn en_marche(pid: u32) -> String {
        crate::tr!("Actif (pid {pid})", pid = pid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// HIG TYP-06 et TYP-14 : chaque état a un terme à lui, qui commence par
    /// une majuscule quand il se lit seul et qui est traduit.
    #[test]
    fn un_terme_par_etat_majuscule_et_traduit() {
        let tous = [
            Etat::Active,
            Etat::Inactive,
            Etat::Arrete,
            Etat::Abandonne,
            Etat::EnErreur,
            Etat::Coeur,
            Etat::Connecte,
            Etat::NonConnecte,
            Etat::Installe,
            Etat::Introuvable,
            Etat::Integre,
            Etat::Livre,
            Etat::Tiers,
        ];
        let mut termes: Vec<&str> = tous.iter().map(|e| e.terme()).collect();
        termes.push("Actif (pid {pid})");
        for t in &termes {
            let initiale = t.chars().next().expect("terme non vide");
            assert!(initiale.is_uppercase(), "« {t} » : majuscule initiale");
            assert!(crate::i18n::has_english(t), "« {t} » : sans traduction");
        }
        termes.sort_unstable();
        termes.dedup();
        assert_eq!(termes.len(), tous.len() + 1, "un terme par état");
        assert_eq!(
            Etat::en_marche(42),
            crate::tr!("Actif (pid {pid})", pid = 42)
        );
    }
}
