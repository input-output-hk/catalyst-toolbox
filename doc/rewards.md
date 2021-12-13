# Rewards data pipeline

The rewards process is an entangled system of data requirements which will 
be listed in the former document.


## Voters rewards

Currently, (as per Fund7) the tool needs:

* The block0 file (bin)
* The amount of rewards to distribute
* The threshold of votes a voter need in order to access such rewards


Algorithm is performed as per:

1. Take all addresses that voted
2. Check which addresses voted more than threshold
3. Calculate reward per voter as `(voter_stake/total_voters_stake)*total_rewards`

A Csv is generated with the following headers:


```
+---------+---------------------------+----------------------------+-------------------------------+
| Address | Stake of the voter (ADA)  | Reward for the voter (ADA) |Reward for the voter (lovelace)|
+---------+---------------------------+----------------------------+-------------------------------+
```

## Proposers reward

Users that propose proposals get a rewards too. We can use the [`proposers_rewards.py`](https://github.com/input-output-hk/catalyst-toolbox#calculate-proposers-rewards) script for that.
The scrip has two modes of operating, online and offline. 
The online mode works with the data living in the vit-servicing-station server.
The offline mode need to load that data manually through some json files. 
Those json files can be downloaded from the vit-servicing-station at any time during the fund.

### Json files needed
1. challenges: from `https://servicing-station.vit.iohk.io/api/v0/challenges`
2. active voteplans: from `https://servicing-station.vit.iohk.io/api/v0/vote/active/plans`
3. proposals: from `https://servicing-station.vit.iohk.io/api/v0/proposals`
4. excluded proposals: a json file with a list of excluded proposals ids `[id, ..idx]`

### Output

The proposers output is csv with several data on it. 
***It is really important***, this output file is used as source of truth for the approved proposals 
(not to be mistaken with funded proposals).

Output csv headers:
* internal_id: proposal internal id (from vss)
* proposal_id: proposal chain id
* proposal: proposal title
* overall_score: proposal impact score
* yes: how many yes votes
* no: how many no votes
* result: yes vs no votes difference
* meets_approval_threshold: **is proposal approved**
* requested_dollars: amount of funding requested
* status: **is proposal funded**
* fund_depletion: fund remaining after proposal depletion
* not_funded_reason: why wasnt the proposal not funded (if applies, over budget or approval threshold)
* link_to_ideascale: url to ideascale proposal page