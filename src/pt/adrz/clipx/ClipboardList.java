package pt.adrz.clipx;

import java.util.LinkedList;
import javax.swing.AbstractListModel;
import javax.swing.JList;
import javax.swing.JTextField;
import javax.swing.ListModel;
import javax.swing.event.DocumentEvent;
import javax.swing.event.DocumentListener;



public class ClipboardList extends JList<String> {
	
	private static final long serialVersionUID = -8105402572524766204L;
	
	// field used to search strings
	private FilterField filterField;

    private int DEFAULT_FIELD_WIDTH = 20;
    
    FilterModel filtermodel;
	
	public ClipboardList() {
		super();
		filtermodel = new FilterModel();
        this.setModel (filtermodel);
        filterField = new FilterField (DEFAULT_FIELD_WIDTH);
	}
	
	public void setModel (ListModel<String> m) {
        if (! (m instanceof FilterModel))
            throw new IllegalArgumentException();
        super.setModel (m);
        //super.setModel(<String>m);
        
    }
	
	public void addItem (String o) {
        ((FilterModel)getModel()).addElement (o);
    }
	
	public void addTo (String o, int index) {
		((FilterModel)getModel()).addElementTo(o,index);
	}
	
	public void remove (int index) {
		((FilterModel)getModel()).remove(index);
	}

    public JTextField getFilterField() {
        return filterField;
    }
    
    public LinkedList<String> getItems() {
    	return ((FilterModel)getModel()).getItems();
    }
	
    public FilterModel getModel() {
    	return this.filtermodel;
    }
    
    
    
    
    
	class FilterModel extends AbstractListModel {
		
		private static final long serialVersionUID = 1L;

		LinkedList<String> items;

        LinkedList<String> filterItems;
        
        
        
        public FilterModel() {

            super();
            items = new LinkedList<String>();
            filterItems = new LinkedList<String>();
        }
        
        
        public Object getElementAt (int index) {
            if (index < filterItems.size())
                return filterItems.get (index);
            else
                return null;
        }
        
        
        
        public LinkedList<String> getItems() {
        	return items;
        }
        
        public int getSize() {
            return filterItems.size();
        }
        
        /**
         * Add an element to the list
         * @param o
         */
        public void addElement (String o) {
            items.add(o);
            refilter();
        }
        
        
        public void addElementTo (String o, int index) {
        	items.add(index, o);
        	refilter();
        }
        
        public void remove(int index) {
        	items.remove(index);
        	refilter();
        }
        
        
        private void refilter() {

            filterItems.clear();

            String term = getFilterField().getText();

            for (int i=0; i<items.size(); i++)
                if (items.get(i).toString().indexOf(term, 0) != -1) 
                	filterItems.add (items.get(i));
                
            fireContentsChanged (this, 0, getSize());  
        }
    }
	
	
	
	
	/**
	 * Class that represents the Textfield used to search.
	 * @author adriano
	 *
	 */
	class FilterField extends JTextField implements DocumentListener {
		
		private static final long serialVersionUID = -6965310542422098581L;

		public FilterField(int width) {
			super(width);
			getDocument().addDocumentListener(this);
		}

		@Override
		public void changedUpdate(DocumentEvent e) {
			((FilterModel)getModel()).refilter();
		}

		@Override
		public void insertUpdate(DocumentEvent e) {
			((FilterModel)getModel()).refilter();
		}

		@Override
		public void removeUpdate(DocumentEvent e) {
			((FilterModel)getModel()).refilter();
		}
	}
}
