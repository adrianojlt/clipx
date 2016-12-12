package pt.adrz.clipx.gui.list;

import java.util.LinkedList;

public class Node {
	
	private String name;
	private LinkedList<String> items;
	
	public Node() {
		
	}

	public Node(String name) {
		this.name = name;
	}
	
	public Node(String name, LinkedList<String> items) {
		this.name = name;
		this.items = items;
	}

	public String getName() {
		return name;
	}
	public void setName(String name) {
		this.name = name;
	}
	public LinkedList<String> getItems() {
		return items;
	}
	public void setItems(LinkedList<String> items) {
		this.items = items;
	}

	@Override
	public String toString() {
		return name;
	}
}
