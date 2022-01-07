import json

proposals = json.load(open('proposals.json'))
users = json.load(open('users.json'))

for user in users:
    new_challenges_ids = []
    for user_proposal in user["proposals"] :
        proposal = next((item for item in proposals if item["id"] == user_proposal), None)
        if (proposal):
            new_challenges_ids.append(proposal["category"])
    user["campaigns"] = new_challenges_ids


with open('users-updated.json', 'w') as outfile:
    json.dump(users, outfile)
