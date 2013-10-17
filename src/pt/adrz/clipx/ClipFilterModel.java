package pt.adrz.clipx;

import java.util.LinkedList;

import javax.swing.AbstractListModel;


public class ClipFilterModel extends AbstractListModel<String> {

	private static final long serialVersionUID = 6652151796589168654L;

	/**
	 * Strings stored
	 */
	private LinkedList<String> items;
	
	/**
	 * Strings shown in the list
	 */
	private LinkedList<String> filterItems;
	
	/**
	 * Search field to filter stored Strings
	 */
	private ClipFilterField filterField;
	
	/**
	 * Constructor
	 * @param field - a reference to the search field
	 */
	public ClipFilterModel(ClipFilterField field) {
		super();
		this.filterField = field;
		items 			= new LinkedList<String>();
		filterItems		= new LinkedList<String>();
	}
	
	/**
	 * Get the items with all strings stored
	 * @return LinkedList with all strings
	 */
	public LinkedList<String> getItems() {
		return items;
	}
	
	/**
	 * Adds an element to the items linked list
	 * refilter will be made
	 * @param str - string to be added
	 */
	public void addElement(String str) {
		items.add(str);
		this.refilter();
	}
	
	/**
	 * add element to a specific place in the list
	 * @param str
	 * @param index
	 */
	public void addElementTo(String str, int index) {
		items.add(index,str);
		this.refilter();
	}
	
	
	/**
	 * Remove a specific item from the list
	 * @param index
	 */
	public void remove(int index) {
		items.remove(index);
		this.refilter();
	}
	
	public String getElement(int index) {
		return items.get(index);
	}
		
	/**
	 * Remove the elemente at "index" and appends a string to the beginning of the list
	 * @param index - the element to be removed
	 * @param str 	- The string to append to the beginning of the items list
	 */
	public void switchVals(int index, String str) {
		items.remove(index);
		items.add(0, str);
		this.refilter();
//		items.set(from, items.set(to, items.get(from)));
	}
	
	
	/**
	 * this method filter all the strings in the list
	 * so that only the strings containing the terms within the search field
	 * would be be shown. 
	 */
	public void refilter() {
		filterItems.clear();
		String term = this.filterField.getText();
		for (int i = 0 ; i < items.size() ; i++) {
			if (items.get(i).toString().indexOf(term,0)!=-1)
				filterItems.add(items.get(i));
		}
		fireContentsChanged(this, 0, getSize());
	}
	
	
	@Override
	public String getElementAt(int index) {
		
		if ( index == -1)
			return null;
		
		if (index < filterItems.size())
            return filterItems.get (index);
        else
            return null;
	}

	@Override
	public int getSize() {
		return filterItems.size();
	}

}
