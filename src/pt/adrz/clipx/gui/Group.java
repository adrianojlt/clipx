package pt.adrz.clipx.gui;

import java.util.LinkedList;

public class Group {
	
	private String name;
	private LinkedList<String> elements;
	
	public Group() {
		
	}

	public String getName() {
		return name;
	}

	public void setName(String name) {
		this.name = name;
	}

	public LinkedList<String> getElements() {
		return elements;
	}

	public void setElements(LinkedList<String> elements) {
		this.elements = elements;
	}
}
