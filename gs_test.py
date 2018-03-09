from query.gsquery import GoldsrcQuery

server = GoldsrcQuery('77.220.180.87', 27015, 10)
challenge = server.get_challenge()

print(server.a2s_info())
print(server.a2s_player(challenge))
print(server.a2s_rules(challenge))
