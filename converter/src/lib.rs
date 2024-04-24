use std::collections::BTreeMap;
use std::str::FromStr;

use erreur::*;
use serde_json::{from_str, from_value, Value};
use svarog_algo_flat::elgamal_secp256k1::{
    KeystoreElgamal, PaillierKey2048, ProjectivePoint, Scalar,
};
use svarog_algo_flat::num_bigint::BigInt;
use svarog_grpc::{Algorithm, CoefComs, Curve, Keystore, Scheme};

pub fn convert(old_json: &str) -> Resultat<Keystore> {
    let new = convert_inner(old_json).catch_()?;

    let mut keystore_pb = Keystore::default();
    keystore_pb.i = new.i as u64;
    keystore_pb.ui = new.ui.to_bytes().to_vec();
    keystore_pb.xi = new.xi.to_bytes().to_vec();
    for (i, coef_com_vec) in new.vss_scheme.iter() {
        let mut coef_com_vec_pb = CoefComs::default();
        for coef_com in coef_com_vec.iter() {
            let coef_com_pub = coef_com.to33bytes().to_vec();
            coef_com_vec_pb.values.push(coef_com_pub);
        }
        keystore_pb.vss_scheme.insert(*i as u64, coef_com_vec_pb);
    }
    keystore_pb.xpub = new.xpub().catch_()?;
    let misc = (new.paillier_key.clone(), new.paillier_n_dict.clone());
    let misc_bytes = serde_pickle::to_vec(&misc, Default::default()).catch_()?;
    keystore_pb.algo = Some(Algorithm {
        curve: Curve::Secp256k1.into(),
        scheme: Scheme::ElGamal.into(),
    });
    keystore_pb.misc = misc_bytes;

    Ok(keystore_pb)
}

fn convert_inner(old_json: &str) -> Resultat<KeystoreElgamal> {
    let old: Value = from_str(old_json).catch_()?;
    let old = old.as_array().ifnone_()?;

    let mut new = KeystoreElgamal::default();

    new.i = {
        let id = old.get(2).ifnone_()?;
        let id = id.as_u64().ifnone_()?;
        id as usize
    };

    new.ui = {
        let party_key = old.get(0).ifnone_()?;
        let party_key = party_key.as_object().ifnone_()?;
        let ui = party_key.get("u_i").ifnone_()?;
        let ui = ui.as_object().ifnone_()?;
        let ui = ui.get("scalar").ifnone_()?;
        let ui: Vec<u8> = from_value(ui.clone()).catch_()?;
        let new_ui = Scalar::from_bytes_mod_order(&ui);
        new_ui
    };

    new.xi = {
        let signing_key = old.get(1).ifnone_()?;
        let signing_key = signing_key.as_object().ifnone_()?;
        let xi = signing_key.get("x_i").ifnone_()?;
        let xi = xi.as_object().ifnone_()?;
        let xi = xi.get("scalar").ifnone_()?;
        let xi: Vec<u8> = from_value(xi.clone()).catch_()?;
        Scalar::from_bytes_mod_order(&xi)
    };

    new.vss_scheme = {
        let vss_scheme_old = old.get(3).ifnone_()?;
        let vss_scheme_old = vss_scheme_old.as_array().ifnone_()?;
        let mut vss_scheme = BTreeMap::new();
        for (j, coef_coms_old) in vss_scheme_old.iter().enumerate() {
            let j = j + 1;
            let coef_coms_old = coef_coms_old.as_object().ifnone_()?;
            let coef_coms_old = coef_coms_old.get("commitments").ifnone_()?;
            let coef_coms_old = coef_coms_old.as_array().ifnone_()?;

            let mut coef_coms = Vec::new();
            for coef_com_old in coef_coms_old.iter() {
                let coef_com_old = coef_com_old.as_object().ifnone_()?;
                let coef_com_old = coef_com_old.get("point").ifnone_()?;
                let coef_com_old: Vec<u8> = from_value(coef_com_old.clone()).catch_()?;
                let coef_com = ProjectivePoint::from33bytes(&coef_com_old).catch_()?;
                coef_coms.push(coef_com);
            }
            vss_scheme.insert(j, coef_coms);
        }
        vss_scheme
    };

    new.paillier_key = {
        let party_key = old.get(0).ifnone_()?;
        let party_key = party_key.as_object().ifnone_()?;
        let dk = party_key.get("dk").ifnone_()?;
        let dk = dk.as_object().ifnone_()?;
        let p = dk.get("p").ifnone_()?;
        let p = p.as_str().ifnone_()?;
        let p: BigInt = BigInt::from_str(p).catch_()?;
        let q = dk.get("q").ifnone_()?;
        let q = q.as_str().ifnone_()?;
        let q: BigInt = BigInt::from_str(q).catch_()?;
        let new_paillier_key = PaillierKey2048::import(p, q).catch_()?;
        new_paillier_key
    };

    new.paillier_n_dict = {
        let n_vec_old = old.get(4).ifnone_()?;
        let n_vec_old = n_vec_old.as_array().ifnone_()?;
        let mut n_dict = BTreeMap::new();
        for (j, n_old) in n_vec_old.iter().enumerate() {
            let j = j + 1;
            let n_old = n_old.as_object().ifnone_()?;
            let n_old = n_old.get("n").ifnone_()?;
            let n_old = n_old.as_str().ifnone_()?;
            let n: BigInt = BigInt::from_str(n_old).catch_()?;
            n_dict.insert(j, n);
        }
        n_dict
    };

    Ok(new)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use rand::{rngs::OsRng, seq::IteratorRandom};
    use sha2::{Digest, Sha256};
    use svarog_algo_flat::elgamal_secp256k1::{sign, SignatureElgamal};
    use svarog_grpc::SessionConfig;
    use svarog_sesman::SvarogChannel;

    use crate::*;

    const KEYSTORES: [&str; 3] = [
        r#"[{"u_i":{"curve":"secp256k1","scalar":[159,28,160,25,241,160,134,212,88,52,197,59,31,141,193,140,175,223,198,156,105,183,228,170,56,191,101,32,228,155,201,100]},"y_i":{"curve":"secp256k1","point":[3,188,221,191,232,31,192,249,251,79,24,214,173,158,58,130,173,195,85,221,39,89,55,53,31,236,229,23,4,134,241,248,64]},"dk":{"p":"144519320196421578229942353187397320789835487363686004656511762763016459887724566682104366580187692454533533052522180526436396260744826145216715433848635129390376697628687070245511313951815666449855952058327501991387920847139245959252127744098357868091625838637412515293041530446447123232557908781716027823747","q":"162423912167466020637558236047946719448051566011026688962372788864221326298622626910947393262518272043701031067462156244650501577003176709927712757865230713994200543577363898570480413297682005303865311095282095357629712562948842578801623086822248888681048423761728926684290770242458762429284494534875862187177"},"ek":{"n":"23473393370085476595861095346748774887630760195002715122862858363706842155119889810464513224717355554760308754899814135946372542426223376650268153608193286917177002559327187669996305421145955430253475928534348608759307298789654079165905402459759161903582577771928044228939170503061649158015974898140164031644462684405367684443406082293427878829099670406417046973043609162452857894050890836127817161214770222869658832239725916974838073526503714564807601983136099348335493678678160564409868602353477598524877399585585485999080473431692390703472427097506418634121163684951832091605874759644387059373992495921634879492219"},"party_index":2},{"y":{"curve":"secp256k1","point":[3,0,127,54,153,244,219,61,161,81,214,28,48,56,171,142,119,20,104,251,248,73,25,43,80,38,53,45,128,143,105,79,145]},"x_i":{"curve":"secp256k1","scalar":[171,164,33,186,86,196,250,230,141,199,213,197,21,157,147,219,52,82,118,245,241,93,43,31,199,241,45,54,56,206,70,82]}},2,[{"parameters":{"threshold":1,"share_count":3},"commitments":[{"curve":"secp256k1","point":[3,167,56,101,31,186,186,107,181,2,36,37,236,37,234,156,92,47,128,13,176,183,34,88,37,220,209,238,176,232,224,73,25]},{"curve":"secp256k1","point":[3,73,207,56,31,243,94,160,194,40,206,62,141,213,42,241,124,101,243,85,199,221,232,12,213,77,30,235,165,141,44,8,20]}]},{"parameters":{"threshold":1,"share_count":3},"commitments":[{"curve":"secp256k1","point":[3,188,221,191,232,31,192,249,251,79,24,214,173,158,58,130,173,195,85,221,39,89,55,53,31,236,229,23,4,134,241,248,64]},{"curve":"secp256k1","point":[2,64,134,225,249,35,226,250,180,62,21,166,175,25,62,169,31,254,85,32,233,54,90,55,29,176,188,35,33,17,218,108,124]}]},{"parameters":{"threshold":1,"share_count":3},"commitments":[{"curve":"secp256k1","point":[3,104,152,109,172,80,181,128,183,187,230,150,139,93,18,217,190,234,35,240,30,179,74,147,18,163,194,199,154,245,239,137,141]},{"curve":"secp256k1","point":[2,171,18,187,36,7,249,2,154,28,213,78,24,23,232,64,104,156,212,126,49,214,41,1,134,120,77,221,178,109,71,220,34]}]}],[{"n":"30138873334387118954755039585204388732866272135622030996476255424630672316865349999114792934071903736508725998179112824550135290997378758151410250352341656330322772641534760418458809673230258975332764053664958477766020795422424992894301951711378020728402792524479653251949853317809318484539184741057975180651874633716501578869281554574002992325149535767191371862562846183579283286674241838354429335906824440870083429162044606007072987221510583518649231291840109023171903744240844327398724426885275679416573891931542687850641113720602873334262082238732097810063715676022023771419978697692331360045516665079253255249531"},{"n":"23473393370085476595861095346748774887630760195002715122862858363706842155119889810464513224717355554760308754899814135946372542426223376650268153608193286917177002559327187669996305421145955430253475928534348608759307298789654079165905402459759161903582577771928044228939170503061649158015974898140164031644462684405367684443406082293427878829099670406417046973043609162452857894050890836127817161214770222869658832239725916974838073526503714564807601983136099348335493678678160564409868602353477598524877399585585485999080473431692390703472427097506418634121163684951832091605874759644387059373992495921634879492219"},{"n":"12155852436809696805078203204415189681435578078391722867935032215955864901582962714329148060959630388883295001487253528760624395838661673083566160019964581534100299266548674926971686692809217378206950021077828900616805410881432388365547358765046142900941582904094554256632521357578063612974702768158589226099994721955264178546701634154711474292858129218808194479868399771652823013093613395588617166086092862417190203859475414650253512822960714673123883260401006832661043391004258986758434131610379469569975202243737972104494837294368526083098201661723209674592705870599467821271124158033018918409972593022052378028009"}],{"curve":"secp256k1","point":[3,0,127,54,153,244,219,61,161,81,214,28,48,56,171,142,119,20,104,251,248,73,25,43,80,38,53,45,128,143,105,79,145]},[99,231,68,198,84,42,55,52,6,186,43,26,182,141,57,119,232,71,195,136,110,136,213,94,237,91,244,194,5,166,167,61]]"#,
        r#"[{"u_i":{"curve":"secp256k1","scalar":[117,171,141,21,154,238,169,217,209,54,95,218,174,178,225,107,68,100,22,254,243,239,169,79,93,243,161,223,94,178,131,14]},"y_i":{"curve":"secp256k1","point":[3,167,56,101,31,186,186,107,181,2,36,37,236,37,234,156,92,47,128,13,176,183,34,88,37,220,209,238,176,232,224,73,25]},"dk":{"p":"177665742072309883143467484627460392168675407867553005055209690936323245129946970113622282374513552950392495539977130395122209421634989098303385066068787471594944976391556730052086847224184608975077203506229585761668228534324865528289438254620902812449724422907417462277206592919132306874119761553570067724503","q":"169638068559782392041081557061917979378195815103918825733231657988531521259027151598927887603370261956937564433319566859191167240335426293622012511758355757483387927669873186820603739914786303447534151323220048229489869637892014009688302004341594877589000041514756074385199304903786276468741451274701458087677"},"ek":{"n":"30138873334387118954755039585204388732866272135622030996476255424630672316865349999114792934071903736508725998179112824550135290997378758151410250352341656330322772641534760418458809673230258975332764053664958477766020795422424992894301951711378020728402792524479653251949853317809318484539184741057975180651874633716501578869281554574002992325149535767191371862562846183579283286674241838354429335906824440870083429162044606007072987221510583518649231291840109023171903744240844327398724426885275679416573891931542687850641113720602873334262082238732097810063715676022023771419978697692331360045516665079253255249531"},"party_index":1},{"y":{"curve":"secp256k1","point":[3,0,127,54,153,244,219,61,161,81,214,28,48,56,171,142,119,20,104,251,248,73,25,43,80,38,53,45,128,143,105,79,145]},"x_i":{"curve":"secp256k1","scalar":[135,76,38,244,84,18,150,189,107,67,186,79,247,89,121,115,140,35,117,208,121,92,229,90,120,237,184,110,26,130,127,204]}},1,[{"parameters":{"threshold":1,"share_count":3},"commitments":[{"curve":"secp256k1","point":[3,167,56,101,31,186,186,107,181,2,36,37,236,37,234,156,92,47,128,13,176,183,34,88,37,220,209,238,176,232,224,73,25]},{"curve":"secp256k1","point":[3,73,207,56,31,243,94,160,194,40,206,62,141,213,42,241,124,101,243,85,199,221,232,12,213,77,30,235,165,141,44,8,20]}]},{"parameters":{"threshold":1,"share_count":3},"commitments":[{"curve":"secp256k1","point":[3,188,221,191,232,31,192,249,251,79,24,214,173,158,58,130,173,195,85,221,39,89,55,53,31,236,229,23,4,134,241,248,64]},{"curve":"secp256k1","point":[2,64,134,225,249,35,226,250,180,62,21,166,175,25,62,169,31,254,85,32,233,54,90,55,29,176,188,35,33,17,218,108,124]}]},{"parameters":{"threshold":1,"share_count":3},"commitments":[{"curve":"secp256k1","point":[3,104,152,109,172,80,181,128,183,187,230,150,139,93,18,217,190,234,35,240,30,179,74,147,18,163,194,199,154,245,239,137,141]},{"curve":"secp256k1","point":[2,171,18,187,36,7,249,2,154,28,213,78,24,23,232,64,104,156,212,126,49,214,41,1,134,120,77,221,178,109,71,220,34]}]}],[{"n":"30138873334387118954755039585204388732866272135622030996476255424630672316865349999114792934071903736508725998179112824550135290997378758151410250352341656330322772641534760418458809673230258975332764053664958477766020795422424992894301951711378020728402792524479653251949853317809318484539184741057975180651874633716501578869281554574002992325149535767191371862562846183579283286674241838354429335906824440870083429162044606007072987221510583518649231291840109023171903744240844327398724426885275679416573891931542687850641113720602873334262082238732097810063715676022023771419978697692331360045516665079253255249531"},{"n":"23473393370085476595861095346748774887630760195002715122862858363706842155119889810464513224717355554760308754899814135946372542426223376650268153608193286917177002559327187669996305421145955430253475928534348608759307298789654079165905402459759161903582577771928044228939170503061649158015974898140164031644462684405367684443406082293427878829099670406417046973043609162452857894050890836127817161214770222869658832239725916974838073526503714564807601983136099348335493678678160564409868602353477598524877399585585485999080473431692390703472427097506418634121163684951832091605874759644387059373992495921634879492219"},{"n":"12155852436809696805078203204415189681435578078391722867935032215955864901582962714329148060959630388883295001487253528760624395838661673083566160019964581534100299266548674926971686692809217378206950021077828900616805410881432388365547358765046142900941582904094554256632521357578063612974702768158589226099994721955264178546701634154711474292858129218808194479868399771652823013093613395588617166086092862417190203859475414650253512822960714673123883260401006832661043391004258986758434131610379469569975202243737972104494837294368526083098201661723209674592705870599467821271124158033018918409972593022052378028009"}],{"curve":"secp256k1","point":[3,0,127,54,153,244,219,61,161,81,214,28,48,56,171,142,119,20,104,251,248,73,25,43,80,38,53,45,128,143,105,79,145]},[99,231,68,198,84,42,55,52,6,186,43,26,182,141,57,119,232,71,195,136,110,136,213,94,237,91,244,194,5,166,167,61]]"#,
        r#"[{"u_i":{"curve":"secp256k1","scalar":[78,43,254,254,196,209,1,230,31,84,121,197,10,212,188,18,170,95,115,246,82,253,177,215,83,9,155,50,137,30,174,21]},"y_i":{"curve":"secp256k1","point":[3,104,152,109,172,80,181,128,183,187,230,150,139,93,18,217,190,234,35,240,30,179,74,147,18,163,194,199,154,245,239,137,141]},"dk":{"p":"132200702950717693943805772232346223907838643596114467614074630182382032614396716976370914186252281516502982534229592573520579829072211309014777161386153433798482907160555548226223578077869034886608997499770311609199337568328119799781982075613788915821398661931651008639826037737726221218828567507106035773749","q":"91949983362351742010052616301772818698242426566752181424203243540077277108567689417896997953884926300818104385461015402637582711861572319084029607494419086377496382531884194555547038811442819461440085701153170290017089854098766004904923568164776988658589395096103243844344303380832529188906643365430230320741"},"ek":{"n":"12155852436809696805078203204415189681435578078391722867935032215955864901582962714329148060959630388883295001487253528760624395838661673083566160019964581534100299266548674926971686692809217378206950021077828900616805410881432388365547358765046142900941582904094554256632521357578063612974702768158589226099994721955264178546701634154711474292858129218808194479868399771652823013093613395588617166086092862417190203859475414650253512822960714673123883260401006832661043391004258986758434131610379469569975202243737972104494837294368526083098201661723209674592705870599467821271124158033018918409972593022052378028009"},"party_index":3},{"y":{"curve":"secp256k1","point":[3,0,127,54,153,244,219,61,161,81,214,28,48,56,171,142,119,20,104,251,248,73,25,43,80,38,53,45,128,143,105,79,145]},"x_i":{"curve":"secp256k1","scalar":[207,252,28,128,89,119,95,15,176,75,241,58,51,225,174,66,220,129,120,27,105,93,112,229,22,244,161,254,87,26,12,216]}},3,[{"parameters":{"threshold":1,"share_count":3},"commitments":[{"curve":"secp256k1","point":[3,167,56,101,31,186,186,107,181,2,36,37,236,37,234,156,92,47,128,13,176,183,34,88,37,220,209,238,176,232,224,73,25]},{"curve":"secp256k1","point":[3,73,207,56,31,243,94,160,194,40,206,62,141,213,42,241,124,101,243,85,199,221,232,12,213,77,30,235,165,141,44,8,20]}]},{"parameters":{"threshold":1,"share_count":3},"commitments":[{"curve":"secp256k1","point":[3,188,221,191,232,31,192,249,251,79,24,214,173,158,58,130,173,195,85,221,39,89,55,53,31,236,229,23,4,134,241,248,64]},{"curve":"secp256k1","point":[2,64,134,225,249,35,226,250,180,62,21,166,175,25,62,169,31,254,85,32,233,54,90,55,29,176,188,35,33,17,218,108,124]}]},{"parameters":{"threshold":1,"share_count":3},"commitments":[{"curve":"secp256k1","point":[3,104,152,109,172,80,181,128,183,187,230,150,139,93,18,217,190,234,35,240,30,179,74,147,18,163,194,199,154,245,239,137,141]},{"curve":"secp256k1","point":[2,171,18,187,36,7,249,2,154,28,213,78,24,23,232,64,104,156,212,126,49,214,41,1,134,120,77,221,178,109,71,220,34]}]}],[{"n":"30138873334387118954755039585204388732866272135622030996476255424630672316865349999114792934071903736508725998179112824550135290997378758151410250352341656330322772641534760418458809673230258975332764053664958477766020795422424992894301951711378020728402792524479653251949853317809318484539184741057975180651874633716501578869281554574002992325149535767191371862562846183579283286674241838354429335906824440870083429162044606007072987221510583518649231291840109023171903744240844327398724426885275679416573891931542687850641113720602873334262082238732097810063715676022023771419978697692331360045516665079253255249531"},{"n":"23473393370085476595861095346748774887630760195002715122862858363706842155119889810464513224717355554760308754899814135946372542426223376650268153608193286917177002559327187669996305421145955430253475928534348608759307298789654079165905402459759161903582577771928044228939170503061649158015974898140164031644462684405367684443406082293427878829099670406417046973043609162452857894050890836127817161214770222869658832239725916974838073526503714564807601983136099348335493678678160564409868602353477598524877399585585485999080473431692390703472427097506418634121163684951832091605874759644387059373992495921634879492219"},{"n":"12155852436809696805078203204415189681435578078391722867935032215955864901582962714329148060959630388883295001487253528760624395838661673083566160019964581534100299266548674926971686692809217378206950021077828900616805410881432388365547358765046142900941582904094554256632521357578063612974702768158589226099994721955264178546701634154711474292858129218808194479868399771652823013093613395588617166086092862417190203859475414650253512822960714673123883260401006832661043391004258986758434131610379469569975202243737972104494837294368526083098201661723209674592705870599467821271124158033018918409972593022052378028009"}],{"curve":"secp256k1","point":[3,0,127,54,153,244,219,61,161,81,214,28,48,56,171,142,119,20,104,251,248,73,25,43,80,38,53,45,128,143,105,79,145]},[99,231,68,198,84,42,55,52,6,186,43,26,182,141,57,119,232,71,195,136,110,136,213,94,237,91,244,194,5,166,167,61]]"#,
    ];

    const SESMAN_URL: &str = "http://127.0.0.1:9000";

    #[tokio::test]
    async fn test_convert() -> Resultat<()> {
        // 因为绕过peer直接调用算法接口, 所以不必填写会话配置.
        let chan = SvarogChannel::new_session(&SessionConfig::default(), SESMAN_URL)
            .await
            .catch_()?;

        let mut keystores = BTreeMap::new();
        for (_i, json) in KEYSTORES.iter().enumerate() {
            let keystore = convert_inner(json).catch_()?;
            keystores.insert(keystore.i, keystore);
        }

        let signers = {
            let mut rng = OsRng;
            let signers: Vec<usize> = keystores.keys().cloned().choose_multiple(&mut rng, 2);
            let signers: BTreeSet<usize> = signers.into_iter().collect();
            signers
        };

        let task = {
            let mut hasher = Sha256::new();
            hasher.update("Le vieux piano de la plage ne joue qu'en Fa qu'en fatigué");
            let hmsg = hasher.finalize().to_vec();
            let dpath = "m/1/2/3/4".to_owned();
            (hmsg, dpath)
        };

        let mut sign_threads = Vec::new();
        for i in signers.iter() {
            let chan = chan.clone();
            let keystore = keystores.get(i).ifnone_()?.clone();
            let signers = signers.clone();
            let (hmsg, dpath) = task.clone();
            let future = sign(chan, signers, keystore, hmsg, dpath);
            let thread = tokio::spawn(future);
            sign_threads.push(thread);
        }
        let mut sig: Option<SignatureElgamal> = None;
        for thread in sign_threads {
            let _sig = thread.await.catch("", "Panic")?.catch("", "Exception")?;
            match sig {
                None => sig = Some(_sig),
                Some(ref sig) => assert_throw!(sig == &_sig),
            }
        }

        Ok(())
    }
}
