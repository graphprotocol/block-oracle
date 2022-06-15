import { Entity, store } from "@graphprotocol/graph-ts";

export namespace DirtyChanges {
	let removals: Set<EntityId> = new Set();
	let saves: Map<EntityId, Entity> = new Map();

	export function get<T>(entityName: string, id: string): T | null {
		let dirty = saves.get(new EntityId(entityName, id));
		if (dirty == null) {
			return changetype<T | null>(store.get(entityName, id));
		} else {
			return changetype<T>(dirty!);
		}
	}

	export function remove(entityName: string, entity: Entity): void {
		let id = entity.get("id")!.toString();
		removals.add(new EntityId(entityName, id));
	}

	export function set(entityName: string, entity: Entity): void {
		let id = entity.get("id")!.toString();
		saves.set(new EntityId(entityName, id), entity);
	}

	export function persist(): void {
		let removalsValues = removals.values();
		for (let i = 0; i < removals.size; i++) {
			store.remove(removalsValues[i].entityName, removalsValues[i].id);
		}

		let savesKeys = saves.keys();
		let savesValues = saves.values();
		for (let i = 0; i < saves.size; i++) {
			let entity = savesValues[i];
			let id = entity.get("id")!.toString();
			assert(id == savesKeys[i].id, "Entity id mismatch when persisting dirty changes. This is a bug!");

			store.set(savesKeys[i].entityName, id, entity);
		}

		// Clean everything up so it's ready to be used again.
		removals.clear();
		saves.clear();
	}
}

class EntityId {
	entityName: string;
	id: string;

	constructor(entityName: string, id: string) {
		this.entityName = entityName;
		this.id = id;
	}
}
