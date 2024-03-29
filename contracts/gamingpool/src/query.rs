use cosmwasm_std::{Deps, Order, StdError, StdResult, Storage, Uint128};

use crate::contract::{
    DUMMY_WALLET, INITIAL_TEAM_POINTS, INITIAL_TEAM_RANK, UNCLAIMED_REFUND, UNCLAIMED_REWARD,
};
use crate::execute::query_platform_fees;
use crate::state::{
    FeeDetails, GameDetails, GameResult, PoolDetails, PoolTeamDetails, PoolTypeDetails,
    SwapBalanceDetails, CONFIG, FEE_WALLET, GAME_DETAILS, GAME_RESULT_DUMMY, POOL_DETAILS,
    POOL_TEAM_DETAILS, POOL_TYPE_DETAILS, SWAP_BALANCE_INFO,
};

pub fn query_get_fee_wallet(deps: Deps) -> StdResult<String> {
    let address = FEE_WALLET.load(deps.storage)?;
    return Ok(address);
}

pub fn query_pool_type_details(
    storage: &dyn Storage,
    pool_type: String,
) -> StdResult<PoolTypeDetails> {
    let ptd = POOL_TYPE_DETAILS.may_load(storage, pool_type)?;
    match ptd {
        Some(ptd) => return Ok(ptd),
        None => return Err(StdError::generic_err("No pool type details found")),
    };
}

pub fn query_total_fees(deps: Deps, amount: Uint128) -> StdResult<FeeDetails> {
    let config = CONFIG.load(deps.storage)?;
    let result = query_platform_fees(amount, config.platform_fee, config.transaction_fee)?;
    return Ok(result);
}

pub fn query_all_pool_type_details(storage: &dyn Storage) -> StdResult<Vec<PoolTypeDetails>> {
    let mut all_pool_types = Vec::new();
    let all_pool_type_names: Vec<String> = POOL_TYPE_DETAILS
        .keys(storage, None, None, Order::Ascending)
        .map(|k| k.unwrap())
        .collect();
    for ptn in all_pool_type_names {
        let pool_type = POOL_TYPE_DETAILS.load(storage, ptn)?;
        all_pool_types.push(pool_type);
    }
    return Ok(all_pool_types);
}

pub fn query_pool_team_details(
    storage: &dyn Storage,
    pool_id: String,
    user: String,
) -> StdResult<Vec<PoolTeamDetails>> {
    let ptd = POOL_TEAM_DETAILS.may_load(storage, (&*pool_id, user.as_ref()))?;
    match ptd {
        Some(ptd) => return Ok(ptd),
        None => return Err(StdError::generic_err("No team details found")),
    };
}

pub fn query_all_teams(
    storage: &dyn Storage,
    users: Vec<String>,
) -> StdResult<Vec<PoolTeamDetails>> {
    let mut all_teams = Vec::new();
    let all_pools: Vec<String> = POOL_DETAILS
        .keys(storage, None, None, Order::Ascending)
        .map(|k| k.unwrap())
        .collect();
    for pool_id in all_pools {
        for user in users.clone() {
            let team_details = POOL_TEAM_DETAILS.load(storage, (&*pool_id.clone(), user.as_ref()));
            match team_details {
                Ok(teams) => {
                    for team in teams {
                        all_teams.push(team);
                    }
                }
                Err(_) => {
                    //     pass
                }
            }
        }
    }
    return Ok(all_teams);
}

pub fn query_reward(storage: &dyn Storage, gamer: String) -> StdResult<Uint128> {
    let mut user_reward = Uint128::zero();
    // Get all pools
    let all_pools: Vec<String> = POOL_DETAILS
        .keys(storage, None, None, Order::Ascending)
        .map(|k| k.unwrap())
        .collect();
    for pool_id in all_pools {
        // Get the existing teams for this pool
        let mut teams = Vec::new();
        let all_teams = POOL_TEAM_DETAILS.may_load(storage, (&*pool_id.clone(), gamer.as_ref()))?;
        match all_teams {
            Some(some_teams) => {
                teams = some_teams;
            }
            None => {}
        }
        for team in teams {
            if gamer == team.gamer_address && team.claimed_reward == UNCLAIMED_REWARD {
                user_reward += team.reward_amount;
            }
        }
    }
    return Ok(user_reward);
}

pub fn query_refund(storage: &dyn Storage, gamer: String) -> StdResult<Uint128> {
    let mut user_refund = Uint128::zero();
    // Get all pools
    let all_pools: Vec<String> = POOL_DETAILS
        .keys(storage, None, None, Order::Ascending)
        .map(|k| k.unwrap())
        .collect();
    for pool_id in all_pools {
        let mut pool_details: PoolDetails = Default::default();
        let pd = POOL_DETAILS.load(storage, pool_id.clone());
        match pd {
            Ok(some) => {
                pool_details = some;
            }
            Err(_) => {
                continue;
            }
        }
        if !pool_details.pool_refund_status {
            continue;
        }
        let ptd = POOL_TYPE_DETAILS.load(storage, pool_details.pool_type)?;
        let mut teams = Vec::new();
        let all_teams = POOL_TEAM_DETAILS.may_load(storage, (&*pool_id.clone(), gamer.as_ref()))?;
        match all_teams {
            Some(some_teams) => {
                teams = some_teams;
            }
            None => {}
        }
        for team in teams {
            if gamer == team.gamer_address && team.claimed_refund == UNCLAIMED_REFUND {
                user_refund += ptd.pool_fee;
            }
        }
    }
    return Ok(user_refund);
}

pub fn query_game_result(
    deps: Deps,
    gamer: String,
    pool_id: String,
    team_id: String,
) -> StdResult<GameResult> {
    let config = CONFIG.load(deps.storage)?;
    let game_id = config.game_id;

    let mut reward_amount = Uint128::zero();
    let mut refund_amount = Uint128::zero();
    let mut team_rank = INITIAL_TEAM_RANK;
    let mut team_points = INITIAL_TEAM_POINTS;

    let dummy_wallet = String::from(DUMMY_WALLET);
    let address = deps.api.addr_validate(dummy_wallet.clone().as_str())?;
    let grd = GAME_RESULT_DUMMY.may_load(deps.storage, &address)?;
    let mut game_result;
    match grd {
        Some(grd) => {
            game_result = grd;
        }
        None => return Err(StdError::generic_err("No game result details found")),
    }

    // Get the existing teams for this pool
    let mut teams = Vec::new();
    let all_teams =
        POOL_TEAM_DETAILS.may_load(deps.storage, (&*pool_id.clone(), gamer.as_ref()))?;
    match all_teams {
        Some(some_teams) => {
            teams = some_teams;
        }
        None => {}
    }
    for team in teams {
        if gamer == team.gamer_address
            && team_id == team.team_id
            && game_id == team.game_id
            && pool_id == team.pool_id
        {
            team_rank = team.team_rank;
            team_points = team.team_points;
            if team.claimed_reward == UNCLAIMED_REWARD {
                reward_amount += team.reward_amount;
            }
            if team.claimed_refund == UNCLAIMED_REFUND {
                refund_amount += team.refund_amount;
            }
        }
    }
    game_result.gamer_address = gamer.clone();
    game_result.team_id = team_id.clone();
    game_result.reward_amount = reward_amount;
    return Ok(game_result);
}

pub fn query_pool_details(storage: &dyn Storage, pool_id: String) -> StdResult<PoolDetails> {
    let pd = POOL_DETAILS.may_load(storage, pool_id.clone())?;
    match pd {
        Some(pd) => return Ok(pd),
        None => return Err(StdError::generic_err("No pool details found")),
    };
}

pub fn get_team_count_for_user_in_pool_type(
    storage: &dyn Storage,
    gamer: String,
    game_id: String,
    pool_type: String,
) -> StdResult<u32> {
    let mut count = 0;
    let all_pools: Vec<(String, String)> = POOL_TEAM_DETAILS
        .keys(storage, None, None, Order::Ascending)
        .map(|k| k.unwrap())
        .collect();
    for (pool_id, g) in all_pools {
        let team_details = POOL_TEAM_DETAILS.load(storage, (&*pool_id.clone(), gamer.as_ref()))?;
        for team in team_details {
            if team.pool_type == pool_type
                && team.game_id == game_id
                && team.gamer_address == gamer
                && team.pool_id == pool_id
            {
                count += 1;
            }
        }
    }
    println!("Team count for user in given pool type : {:?}", count);
    return Ok(count);
}

pub fn query_game_details(storage: &dyn Storage) -> StdResult<GameDetails> {
    let config = CONFIG.load(storage)?;
    let game_id = config.game_id;

    let game_detail = GAME_DETAILS.may_load(storage, game_id)?;
    match game_detail {
        Some(game_detail) => return Ok(game_detail),
        None => return Err(StdError::generic_err("No Game detail found")),
    };
}

pub fn query_team_details(
    storage: &dyn Storage,
    pool_id: String,
    team_id: String,
    gamer: String,
) -> StdResult<PoolTeamDetails> {
    let team_details = POOL_TEAM_DETAILS.load(storage, (&*pool_id.clone(), gamer.as_ref()))?;
    for team in team_details {
        if team.team_id == team_id.to_string() {
            return Ok(team.clone());
        }
    }
    return Err(StdError::generic_err("Pool Team Details not found"));
}

pub fn query_all_pools_in_game(storage: &dyn Storage) -> StdResult<Vec<PoolDetails>> {
    let config = CONFIG.load(storage)?;
    let game_id = config.game_id;

    let mut all_pool_details = Vec::new();
    let all_pools: Vec<String> = POOL_DETAILS
        .keys(storage, None, None, Order::Ascending)
        .map(|k| k.unwrap())
        .collect();
    for pool_name in all_pools {
        let pool_details = POOL_DETAILS.load(storage, pool_name)?;
        if pool_details.game_id == game_id {
            all_pool_details.push(pool_details);
        }
    }
    return Ok(all_pool_details);
}

pub fn query_pool_collection(storage: &dyn Storage, pool_id: String) -> StdResult<Uint128> {
    let pd = POOL_DETAILS.may_load(storage, pool_id.clone())?;
    let pool;
    match pd {
        Some(pd) => pool = pd,
        None => return Err(StdError::generic_err("No pool details found")),
    };

    let ptd = POOL_TYPE_DETAILS.may_load(storage, pool.pool_type.clone())?;
    let pool_type;
    match ptd {
        Some(ptd) => {
            pool_type = ptd;
        }
        None => return Err(StdError::generic_err("No pool type details found")),
    };

    let pool_collection = pool_type
        .pool_fee
        .checked_mul(Uint128::from(pool.current_teams_count))
        .unwrap_or_default();
    return Ok(pool_collection);
}

pub fn query_swap_data_for_pool(
    storage: &dyn Storage,
    pool_id: String,
) -> StdResult<SwapBalanceDetails> {
    let info = SWAP_BALANCE_INFO.load(storage, pool_id)?;
    return Ok(info);
}
