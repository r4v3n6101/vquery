from query.gsquery import GoldsrcQuery

server = GoldsrcQuery('46.174.52.15', 27333, 'source', 10)
challenge = server.get_challenge()

print(server.a2s_info())
print(server.a2s_player(challenge))
print(server.a2s_rules(challenge))
