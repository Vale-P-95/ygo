-- DuelForge: utility.lua's Auxiliary.SelectUnselectLoop with the same
-- answers, for less work. Upstream reaches every group once per order of
-- its cards, and enters Lua from the engine (Group.IsExists) for every
-- card it tries: a Ritual Summon by Level sum (Ritual.Filter) over two
-- dozen materials in the Deck enters Lua millions of times in one request.
--
-- Here each group is tried once, and the cards are walked in Lua. Whether
-- a group can be extended depends on the group alone: rescon gets the
-- group and the card just added, which is in it. The memory lives for one
-- call from outside, so the duel state cannot change under it.
local function loop(c,sg,cards,mg,e,tp,minc,maxc,rescon,seen,ids)
	local res=not rescon
	if #sg>=maxc then return false end
	sg:AddCard(c)
	if rescon then
		local stop
		res,stop=rescon(sg,e,tp,mg,c)
		if stop then
			sg:RemoveCard(c)
			return false
		end
	end
	if #sg<minc or (#sg<maxc and not res) then
		-- ids: the card ids of sg, sorted, as the key of the group
		local id=c:GetCardID()
		local pos=#ids+1
		while pos>1 and ids[pos-1]>id do
			ids[pos]=ids[pos-1]
			pos=pos-1
		end
		ids[pos]=id
		local key=table.concat(ids,",")
		res=seen[key]
		if res==nil then
			res=false
			for _,nc in ipairs(cards) do
				if not sg:IsContains(nc) and loop(nc,sg,cards,mg,e,tp,minc,maxc,rescon,seen,ids) then
					res=true
					break
				end
			end
			seen[key]=res
		end
		table.remove(ids,pos)
	end
	sg:RemoveCard(c)
	return res
end
function Auxiliary.SelectUnselectLoop(c,sg,mg,e,tp,minc,maxc,rescon)
	if #sg>=maxc then return false end
	local cards,ids={},{}
	mg:Filter(function(tc) cards[#cards+1]=tc return false end,nil)
	sg:Filter(function(tc) ids[#ids+1]=tc:GetCardID() return false end,nil)
	table.sort(ids)
	return loop(c,sg,cards,mg,e,tp,minc,maxc,rescon,{},ids)
end
