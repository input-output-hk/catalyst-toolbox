import json

users = json.load(open('users.json'))
vcas = json.load(open('vcas.json'))
for vca in vcas:
    related_ca = next((item for item in users if item['id'] == vca['ca_id']), None)
    if (related_ca):
        vca['proposals'] = related_ca['proposals']
        vca['campaigns_as_proposers'] = related_ca['campaigns']
    else:
        print("{} not in ca list".format(vca['ca_id']))

with open('vcas.json', 'w') as f:
    json.dump(vcas, f, indent=4)
